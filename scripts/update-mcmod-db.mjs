#!/usr/bin/env node
import { gunzipSync } from 'node:zlib'
import { writeFileSync, renameSync, existsSync, statSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = resolve(__dirname, '..')
const targetFile = resolve(root, 'crates/mclr-comp/data/mcmod.buf')

const API_URL = 'https://api.github.com/repos/LuLu-ling/mcmod-data/releases/latest'

export async function updateMcmodDatabase(options = {}) {
  const force = Boolean(options.force)
  console.log('Checking latest mcmod dictionary release from LuLu-ling/mcmod-data...')

  let release
  try {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), 10000)
    const resp = await fetch(API_URL, {
      signal: controller.signal,
      headers: { 'User-Agent': 'MCLR-Updater/1.0' },
    })
    clearTimeout(timer)
    if (!resp.ok) {
      throw new Error(`GitHub API returned HTTP ${resp.status}`)
    }
    release = await resp.json()
  } catch (err) {
    console.warn(`[mcmod-db] Could not query GitHub release metadata: ${err.message}`)
    console.warn('[mcmod-db] Keeping existing local dictionary.')
    return false
  }

  const tag = release.tag_name
  const asset = release.assets?.find((a) => a.name === 'mcmod.buf')
  if (!asset || !asset.browser_download_url) {
    console.warn('[mcmod-db] Release has no mcmod.buf asset. Skipping.')
    return false
  }

  // Check if existing file has identical size and force flag is off
  if (!force && existsSync(targetFile)) {
    const localStat = statSync(targetFile)
    if (localStat.size === asset.size) {
      console.log(`[mcmod-db] Local dictionary is already up-to-date (tag: ${tag}, size: ${asset.size} bytes).`)
      return true
    }
  }

  const downloadUrls = [
    asset.browser_download_url,
    `https://ghfast.top/${asset.browser_download_url}`,
    `https://gh.ddlc.top/${asset.browser_download_url}`,
    `https://gh.api.99988866.xyz/${asset.browser_download_url}`,
  ]

  let downloadedBuf = null
  for (const url of downloadUrls) {
    try {
      console.log(`[mcmod-db] Fetching dictionary from ${url.slice(0, 60)}...`)
      const controller = new AbortController()
      const timer = setTimeout(() => controller.abort(), 20000)
      const resp = await fetch(url, {
        signal: controller.signal,
        headers: { 'User-Agent': 'MCLR-Updater/1.0' },
      })
      clearTimeout(timer)
      if (!resp.ok) continue
      const raw = Buffer.from(await resp.arrayBuffer())
      if (raw.length > 100000 && raw[0] === 0x1f && raw[1] === 0x8b) {
        downloadedBuf = raw
        break
      }
    } catch {
      // Try next mirror
    }
  }

  if (!downloadedBuf) {
    console.warn('[mcmod-db] All download mirrors failed or timed out. Keeping existing dictionary.')
    return false
  }

  // Validate integrity by decompressing and counting entries
  let decompressed
  try {
    decompressed = gunzipSync(downloadedBuf)
  } catch (err) {
    console.error(`[mcmod-db] Downloaded file is not valid gzip: ${err.message}`)
    return false
  }

  let pos = 0
  let count = 0
  while (pos < decompressed.length) {
    const tagByte = decompressed[pos++]
    if (tagByte === 0x0a) {
      let len = 0
      let shift = 0
      while (pos < decompressed.length) {
        const b = decompressed[pos++]
        len |= (b & 0x7f) << shift
        shift += 7
        if ((b & 0x80) === 0) break
      }
      pos += len
      count++
    } else {
      break
    }
  }

  if (count < 20000) {
    console.error(`[mcmod-db] Dictionary validation failed: parsed only ${count} entries (expected >20000).`)
    return false
  }

  // Atomic write via temp file
  const tmpFile = `${targetFile}.tmp`
  writeFileSync(tmpFile, downloadedBuf)
  renameSync(tmpFile, targetFile)

  console.log(`[mcmod-db] Successfully updated mcmod.buf:`)
  console.log(`  - Upstream release: ${tag}`)
  console.log(`  - Total mod entries: ${count}`)
  console.log(`  - Compressed size: ${(downloadedBuf.length / 1024).toFixed(1)} KB`)
  return true
}

// CLI execution
if (process.argv[1] && process.argv[1].endsWith('update-mcmod-db.mjs')) {
  const force = process.argv.includes('--force')
  updateMcmodDatabase({ force })
    .then((ok) => {
      if (!ok) process.exit(1)
    })
    .catch((err) => {
      console.error(err)
      process.exit(1)
    })
}
