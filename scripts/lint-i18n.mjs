#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { extname, join, relative, resolve } from 'node:path'

const root = process.cwd()
const srcDir = resolve(root, 'src')
const zhCnDir = resolve(srcDir, 'locales/zh-CN')

function loadLocaleTree(dir) {
  const tree = {}
  for (const entry of readdirSync(dir)) {
    if (!entry.endsWith('.json')) continue
    const ns = entry.slice(0, -5)
    tree[ns] = JSON.parse(readFileSync(join(dir, entry), 'utf8'))
  }
  return tree
}

function hasKey(tree, dottedKey) {
  const parts = dottedKey.split('.')
  let cur = tree
  for (const part of parts) {
    if (cur == null || typeof cur !== 'object' || !(part in cur)) {
      return false
    }
    cur = cur[part]
  }
  return typeof cur === 'string'
}

function walkFiles(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    const st = statSync(full)
    if (st.isDirectory()) {
      walkFiles(full, out)
    } else {
      out.push(full)
    }
  }
  return out
}

const localeTree = loadLocaleTree(zhCnDir)
const files = walkFiles(srcDir)
const cjkRegex = /[\u4e00-\u9fff]/
const tCallRegex = /(?:^|[^\w$])(?:\$?t)\(\s*['"]([^'"]+)['"]/g
const keypathRegex = /keypath=["']([^"']+)["']/g

const errors = []

for (const file of files) {
  const rel = relative(srcDir, file).replace(/\\/g, '/')
  const ext = extname(file)
  if (ext !== '.ts' && ext !== '.vue') continue

  const content = readFileSync(file, 'utf8')
  const isLocale = rel.startsWith('locales/')
  const isTest = rel.endsWith('.test.ts') || rel.endsWith('.spec.ts')
  const isMock = rel.endsWith('mock.ts')
  const isBindings = rel === 'ipc/bindings.ts'

  if (!isLocale && !isTest && !isMock && !isBindings) {
    const lines = content.split(/\r?\n/)
    lines.forEach((line, idx) => {
      if (cjkRegex.test(line)) {
        errors.push(`CJK character found in src/${rel}:${idx + 1}: ${line.trim()}`)
      }
    })
  }

  for (const regex of [tCallRegex, keypathRegex]) {
    let match
    while ((match = regex.exec(content)) !== null) {
      const key = match[1]
      // Skip dynamic prefix patterns like 'error.'
      if (key.endsWith('.')) continue
      if (!hasKey(localeTree, key)) {
        errors.push(`Missing zh-CN i18n key "${key}" referenced in src/${rel}`)
      }
    }
  }
}

if (errors.length > 0) {
  console.error('i18n lint failed:')
  for (const err of errors) {
    console.error('  -', err)
  }
  process.exit(1)
}

console.log('i18n lint passed.')
