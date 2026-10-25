#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { parse } from 'vue/compiler-sfc'
import { createGenerator } from 'unocss'
import unoConfig from '../uno.config.ts'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(__dirname, '..')
const srcDir = path.join(rootDir, 'src')
const uiDir = path.join(srcDir, 'ui')

function walkFiles(dir, extRegex, skipTests = true) {
  const results = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (skipTests && entry.isDirectory() && entry.name === '__tests__') continue
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      results.push(...walkFiles(full, extRegex, skipTests))
    } else if (extRegex.test(entry.name) && !entry.name.endsWith('.d.ts')) {
      results.push(full)
    }
  }
  return results
}

function toCamelCase(str) {
  return str.replace(/-([a-zA-Z0-9])/g, (_, c) => c.toUpperCase())
}

function parseComponentContract(filePath) {
  const source = fs.readFileSync(filePath, 'utf8')
  const { descriptor } = parse(source)
  const props = new Set()
  const slots = new Set()

  const scriptContent =
    (descriptor.scriptSetup?.content ?? '') + '\n' + (descriptor.script?.content ?? '')

  // Extract top-level properties inside defineProps<{ ... }>()
  const definePropsIdx = scriptContent.indexOf('defineProps<')
  if (definePropsIdx !== -1) {
    const braceStart = scriptContent.indexOf('{', definePropsIdx)
    if (braceStart !== -1) {
      let depth = 0
      let body = ''
      for (let i = braceStart; i < scriptContent.length; i++) {
        const ch = scriptContent[i]
        if (ch === '{') {
          if (depth > 0) body += ch
          depth++
        } else if (ch === '}') {
          depth--
          if (depth === 0) break
          body += ch
        } else if (depth === 1) {
          body += ch
        }
      }
      // Remove comments
      const cleanBody = body.replace(/\/\/[^\n]*/g, '').replace(/\/\*[\s\S]*?\*\//g, '')
      const propRegex = /(?:^|[;\n])\s*(?:readonly\s+)?(['"]?[a-zA-Z_$][\w$-]*['"]?)\s*\??\s*:/g
      for (const match of cleanBody.matchAll(propRegex)) {
        const rawName = match[1].replace(/^['"]|['"]$/g, '')
        props.add(toCamelCase(rawName))
      }
    }
  }

  function visitTemplate(node) {
    if (!node) return
    if (node.type === 1 && node.tag === 'slot') {
      let slotName = 'default'
      for (const p of node.props || []) {
        if (p.type === 6 && p.name === 'name' && p.value?.content) {
          slotName = p.value.content
        }
      }
      slots.add(slotName)
    }
    for (const child of node.children || []) {
      visitTemplate(child)
    }
  }

  if (descriptor.template?.ast) {
    visitTemplate(descriptor.template.ast)
  }

  return { props, slots }
}

const IGNORED_PROPS = new Set(['class', 'style', 'key', 'ref', 'id', 'is'])

function isIgnoredProp(rawName) {
  if (IGNORED_PROPS.has(rawName)) return true
  if (rawName.startsWith('data-') || rawName.startsWith('aria-')) return true
  return false
}

async function main() {
  // 1. Collect M* component contracts from src/ui/*.vue
  const contracts = new Map()
  for (const entry of fs.readdirSync(uiDir, { withFileTypes: true })) {
    if (entry.isFile() && /^M[A-Z].*\.vue$/.test(entry.name)) {
      const compName = entry.name.replace(/\.vue$/, '')
      contracts.set(compName, parseComponentContract(path.join(uiDir, entry.name)))
    }
  }

  const propSlotErrors = []
  const legacyTokenErrors = []
  const dynamicIconErrors = []
  const unknownColorErrors = []
  const imgErrors = []
  const blockingErrors = []

  const vueFiles = walkFiles(srcDir, /\.vue$/)
  const allCodeFiles = walkFiles(srcDir, /\.(vue|ts|css)$/)

  // (a) Check M* component props & slots in every .vue file
  for (const file of vueFiles) {
    const rel = path.relative(rootDir, file).replace(/\\/g, '/')
    const source = fs.readFileSync(file, 'utf8')
    const { descriptor } = parse(source)
    if (!descriptor.template?.ast) continue

    function visit(node) {
      if (node.type === 1 && contracts.has(node.tag)) {
        const compName = node.tag
        const contract = contracts.get(compName)
        const line = node.loc?.start?.line ?? 1

        for (const p of node.props || []) {
          if (p.type === 6) {
            // Static attribute
            const raw = p.name
            if (isIgnoredProp(raw)) continue
            const camel = toCamelCase(raw)
            if (!contract.props.has(camel)) {
              propSlotErrors.push(`${rel}:${p.loc?.start?.line ?? line} <${compName}> undeclared prop "${raw}"`)
            }
          } else if (p.type === 7) {
            // Directive
            if (p.name === 'bind' && p.arg?.type === 4) {
              const raw = p.arg.content
              if (isIgnoredProp(raw)) continue
              const camel = toCamelCase(raw)
              if (!contract.props.has(camel)) {
                propSlotErrors.push(`${rel}:${p.loc?.start?.line ?? line} <${compName}> undeclared bound prop ":${raw}"`)
              }
            } else if (p.name === 'model') {
              const raw = p.arg?.type === 4 ? p.arg.content : 'modelValue'
              const camel = toCamelCase(raw)
              if (!contract.props.has(camel)) {
                propSlotErrors.push(`${rel}:${p.loc?.start?.line ?? line} <${compName}> undeclared v-model ":${raw}"`)
              }
            } else if (p.name === 'slot') {
              const slotName = p.arg?.type === 4 ? p.arg.content : 'default'
              if (!contract.slots.has(slotName)) {
                propSlotErrors.push(`${rel}:${p.loc?.start?.line ?? line} <${compName}> undeclared slot "#${slotName}"`)
              }
            }
          }
        }

        for (const child of node.children || []) {
          if (child.type === 1 && child.tag === 'template') {
            for (const cp of child.props || []) {
              if (cp.type === 7 && cp.name === 'slot') {
                const slotName = cp.arg?.type === 4 ? cp.arg.content : 'default'
                if (!contract.slots.has(slotName)) {
                  propSlotErrors.push(
                    `${rel}:${cp.loc?.start?.line ?? line} <${compName}> undeclared slot "#${slotName}"`,
                  )
                }
              }
            }
          }
        }
      }
      if (node.type === 1 && node.tag === 'img') {
        const line = node.loc?.start?.line ?? 1
        const names = new Set(
          (node.props || []).map((p) =>
            p.type === 6 ? p.name : p.type === 7 && p.arg?.type === 4 ? p.arg.content : '',
          ),
        )
        if (!names.has('loading') || !names.has('decoding')) {
          imgErrors.push(`${rel}:${line} <img> must have loading and decoding attributes`)
        }
      }
      for (const child of node.children || []) {
        visit(child)
      }
    }

    visit(descriptor.template.ast)
  }

  // (b) Forbidden legacy class/var names & (c) template-built icon names
  const legacyRegex =
    /(?:--c-(?:surface-variant|surface|outline|text-muted|text|on-primary|primary-container)\b|\b(?:text-textMuted|text-text-muted|text-text|text-muted|text-onPrimary|text-on-primary|bg-surfaceVariant|bg-surface-variant|bg-surface(?![-\w])|bg-primaryContainer|bg-primary-container|border-outline)\b)/g
  const dynamicIconRegex = /i-(?:lucide|simple-icons)-\$\{/g

  for (const file of allCodeFiles) {
    const rel = path.relative(rootDir, file).replace(/\\/g, '/')
    const lines = fs.readFileSync(file, 'utf8').split('\n')
    for (let i = 0; i < lines.length; i++) {
      const lineText = lines[i]
      for (const m of lineText.matchAll(legacyRegex)) {
        legacyTokenErrors.push(`${rel}:${i + 1} forbidden legacy token "${m[0]}"`)
      }
      for (const m of lineText.matchAll(dynamicIconRegex)) {
        dynamicIconErrors.push(`${rel}:${i + 1} template-built icon "${m[0]}..."`)
      }
    }
  }

  // (d) UnoCSS extractor check for undefined color/theme utility tokens
  const uno = await createGenerator(unoConfig)
  const colorPrefixes = /^(?:bg|text|border|ring|from|to|via|fill|stroke|divide|outline|shadow|accent)-/
  const cssPropFalsePositives = new Set([
    'border-radius',
    'border-color',
    'border-style',
    'border-width',
    'border-top-width',
    'border-right',
    'border-bottom',
    'border-left',
    'text-decoration',
    'text-align',
    'text-overflow',
    'text-transform',
    'outline-offset',
    'fill-opacity',
    'stroke-width',
    'stroke-linecap',
    'stroke-linejoin',
  ])

  const codeFilesForUno = walkFiles(srcDir, /\.(vue|ts)$/)
  for (const file of codeFilesForUno) {
    const rel = path.relative(rootDir, file).replace(/\\/g, '/')
    const raw = fs.readFileSync(file, 'utf8')
    // Strip <style> blocks and data-testid attributes so CSS properties / testids aren't mistaken for classes
    const stripped = raw
      .replace(/<style[\s\S]*?<\/style>/g, '')
      .replace(/data-testid="[^"]*"/g, '')
      .replace(/(?:fill|stroke)-[a-z-]+=/g, '')

    const extracted = new Set()
    await uno.applyExtractors(stripped, file, extracted)

    for (const token of extracted) {
      if (token.endsWith('-') || token.endsWith(':') || token.endsWith('=')) continue
      const base = token.split(':').pop()
      if (!base || cssPropFalsePositives.has(base)) continue
      if (colorPrefixes.test(base)) {
        const { matched } = await uno.generate([token], { preflights: false })
        if (!matched.has(token)) {
          unknownColorErrors.push(`${rel} unmatched utility token "${token}"`)
        }
      }
    }
  }

  // (f) src-tauri/src/ipc/*.rs #[tauri::command] bodies must wrap sync fs/heavy work in run_blocking
  const bannedCalls = [
    'std::fs::',
    'scan_local_mods(',
    'scan_instances(',
    'list_saves(',
    'list_screenshots(',
    'rename_instance(',
    'duplicate_instance(',
    'delete_instance(',
    'save_instance_config(',
    'backup_save(',
    'export_save_zip(',
    'import_save_zip(',
    'delete_save(',
    'scan_java(',
    'scan_jar_in_jar(',
    'inspect_modpack(',
    'scan_export_candidates(',
    'crop_skin_head(',
    'analyze_crash(',
    'export_crash_report',
    'extract_natives(',
    'read_servers_dat(',
    'add_server(',
    'update_server(',
    'remove_server(',
  ]
  const ipcDir = path.join(rootDir, 'src-tauri', 'src', 'ipc')
  for (const file of walkFiles(ipcDir, /\.rs$/)) {
    const rel = path.relative(rootDir, file).replace(/\\/g, '/')
    const text = fs.readFileSync(file, 'utf8')
    const cmdRe = /#\[tauri::command\][\s\S]*?pub\s+(?:async\s+)?fn\s+(\w+)[\s\S]*?\{/g
    for (const m of text.matchAll(cmdRe)) {
      const fnName = m[1]
      const start = m.index + m[0].length - 1
      let depth = 0
      let end = start
      for (let i = start; i < text.length; i++) {
        if (text[i] === '{') depth++
        else if (text[i] === '}') {
          depth--
          if (depth === 0) {
            end = i
            break
          }
        }
      }
      const body = text.slice(start, end + 1)
      if (body.includes('run_blocking(') || body.includes('spawn_blocking(') || body.includes('tokio::spawn(')) continue
      for (const banned of bannedCalls) {
        if (body.includes(banned)) {
          blockingErrors.push(`${rel} #[tauri::command] ${fnName} calls "${banned}" without run_blocking`)
        }
      }
    }
  }

  const totalErrors =
    propSlotErrors.length +
    legacyTokenErrors.length +
    dynamicIconErrors.length +
    unknownColorErrors.length +
    imgErrors.length +
    blockingErrors.length

  if (totalErrors > 0) {
    console.error(`\n[lint-ui-contracts] Failed with ${totalErrors} violation(s):`)
    if (propSlotErrors.length > 0) {
      console.error(`\n  (a) Undeclared M* component props/slots (${propSlotErrors.length}):`)
      for (const e of propSlotErrors.slice(0, 30)) console.error(`    - ${e}`)
      if (propSlotErrors.length > 30) console.error(`    ... and ${propSlotErrors.length - 30} more`)
    }
    if (legacyTokenErrors.length > 0) {
      console.error(`\n  (b) Forbidden legacy tokens/variables (${legacyTokenErrors.length}):`)
      for (const e of legacyTokenErrors.slice(0, 30)) console.error(`    - ${e}`)
      if (legacyTokenErrors.length > 30) console.error(`    ... and ${legacyTokenErrors.length - 30} more`)
    }
    if (dynamicIconErrors.length > 0) {
      console.error(`\n  (c) Template-built icon names (${dynamicIconErrors.length}):`)
      for (const e of dynamicIconErrors) console.error(`    - ${e}`)
    }
    if (unknownColorErrors.length > 0) {
      console.error(`\n  (d) Unmatched UnoCSS utility tokens (${unknownColorErrors.length}):`)
      for (const e of unknownColorErrors.slice(0, 30)) console.error(`    - ${e}`)
      if (unknownColorErrors.length > 30) console.error(`    ... and ${unknownColorErrors.length - 30} more`)
    }
    if (imgErrors.length > 0) {
      console.error(`\n  (e) <img> missing loading/decoding (${imgErrors.length}):`)
      for (const e of imgErrors) console.error(`    - ${e}`)
    }
    if (blockingErrors.length > 0) {
      console.error(`\n  (f) Sync fs/work in #[tauri::command] without run_blocking (${blockingErrors.length}):`)
      for (const e of blockingErrors) console.error(`    - ${e}`)
    }
    process.exit(1)
  }

  console.log('[lint-ui-contracts] 0 violations (props/slots, legacy tokens, static icons, UnoCSS tokens).')
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
