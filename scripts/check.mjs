#!/usr/bin/env node
import { execSync } from 'node:child_process'

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
run('pnpm gen:bindings')

if (hasCommits()) {
  run('git diff --exit-code src/ipc/bindings.ts')
}

run('pnpm exec vue-tsc --noEmit')
run('pnpm exec vitest run')
run('node scripts/lint-i18n.mjs')
run('pnpm exec vite build')

console.log('\nAll check gates passed.')
