#!/usr/bin/env node
import { execSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'

// Prevent local WireMock test servers (127.0.0.1) from routing through developer/system HTTP proxies.
process.env.NO_PROXY = process.env.NO_PROXY ? `${process.env.NO_PROXY},127.0.0.1,localhost` : '127.0.0.1,localhost';
process.env.no_proxy = process.env.NO_PROXY;

function run(cmd) {
  console.log(`\n> ${cmd}`)
  execSync(cmd, { stdio: 'inherit' })
}

function hasCommits() {
  try {
    execSync('git rev-parse --verify HEAD', { stdio: 'ignore' })
    return true
  } catch {
    return false
  }
}

run('cargo fmt --all --check')
run('cargo clippy --workspace --all-targets -- -D warnings')
run('cargo test --workspace')
const bindingsPath = 'src/ipc/bindings.ts'
const beforeBindings = existsSync(bindingsPath) ? readFileSync(bindingsPath, 'utf8') : null
run('pnpm gen:bindings')
const afterBindings = existsSync(bindingsPath) ? readFileSync(bindingsPath, 'utf8') : null
if (hasCommits() && beforeBindings !== null && beforeBindings !== afterBindings) {
  console.error('src/ipc/bindings.ts was out of date! Re-run pnpm check now that it is updated.')
  process.exit(1)
}

run('pnpm exec oxfmt --check src')
run('pnpm exec oxlint')
run('pnpm exec vue-tsc --noEmit')
run('pnpm exec vitest run')
run('node scripts/lint-i18n.mjs')
run('node scripts/lint-ui-contracts.mjs')
run('pnpm exec vite build')

console.log('\nAll check gates passed.')
