#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const version = process.argv[2]
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error('Usage: node scripts/bump-version.mjs <x.y.z>')
  process.exit(1)
}

const root = process.cwd()

// 1. package.json
const pkgPath = resolve(root, 'package.json')
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'))
pkg.version = version
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8')

// 2. Cargo.toml [workspace.package]
const cargoPath = resolve(root, 'Cargo.toml')
let cargo = readFileSync(cargoPath, 'utf8')
cargo = cargo.replace(
  /(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
  `$1${version}$2`,
)
writeFileSync(cargoPath, cargo, 'utf8')

// 3. src-tauri/tauri.conf.json
const tauriConfPath = resolve(root, 'src-tauri/tauri.conf.json')
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8'))
tauriConf.version = version
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n', 'utf8')

console.log(`Bumped version to ${version}`)
