#!/usr/bin/env node

/**
 * ⚡ Smart Affected Test Runner for bList & fly-common
 *
 * Inspects git diffs (staged, unstaged, or branch diffs against origin/main)
 * and executes ONLY the test suites and linters directly affected by your changes.
 *
 * Usage:
 *   node scripts/test-affected.js
 *   node scripts/test-affected.js --base=origin/main
 *   node scripts/test-affected.js --staged
 *   node scripts/test-affected.js --all
 *   node scripts/test-affected.js --dry-run
 */

const { execSync, spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const args = process.argv.slice(2);
const isDryRun = args.includes('--dry-run');
const runAll = args.includes('--all');
const stagedOnly = args.includes('--staged');
const baseArg = args.find((a) => a.startsWith('--base='));
const baseBranch = baseArg ? baseArg.split('=')[1] : 'origin/main';

function getChangedFiles() {
  if (runAll) return null; // Run everything

  const changed = new Set();

  // 1. Uncommitted working directory & unstaged changes
  try {
    const statusOut = execSync('git status --porcelain', { encoding: 'utf8' }).trim();
    if (statusOut) {
      statusOut.split('\n').forEach((line) => {
        const match = line.match(/^.{2}\s+(.+)$/);
        if (match) {
          const file = match[1].trim().replace(/^"|"$/g, '');
          if (file) changed.add(file);
        }
      });
    }
  } catch (_) {}

  // 2. Staged changes
  if (stagedOnly) {
    return Array.from(changed);
  }

  // 3. Diff against base branch (e.g. origin/main or HEAD~1)
  try {
    const diffBase = execSync(`git diff --name-only ${baseBranch}...HEAD`, { encoding: 'utf8' }).trim();
    if (diffBase) {
      diffBase.split('\n').forEach((f) => {
        const trimmed = f.trim();
        if (trimmed) changed.add(trimmed);
      });
    }
  } catch (_) {
    try {
      const diffHead = execSync('git diff --name-only HEAD~1', { encoding: 'utf8' }).trim();
      if (diffHead) {
        diffHead.split('\n').forEach((f) => {
          const trimmed = f.trim();
          if (trimmed) changed.add(trimmed);
        });
      }
    } catch (_) {}
  }

  return Array.from(changed);
}

function resolveAffectedTests(changedFiles) {
  const plan = {
    frontendUnit: false,
    frontendA11y: false,
    rustFmt: false,
    rustClippy: false,
    rustFull: false,
    rustModules: new Set(),
    e2eSpecs: new Set(),
    description: []
  };

  if (!changedFiles || changedFiles.length === 0) {
    plan.description.push('No changed files detected. Running standard baseline test verification.');
    plan.frontendUnit = true;
    plan.frontendA11y = true;
    plan.rustFull = true;
    plan.rustClippy = true;
    plan.rustFmt = true;
    return plan;
  }

  let hasRustChanges = false;

  for (const file of changedFiles) {
    const norm = file.replace(/\\/g, '/');

    // Root config / dependencies change -> Full verification
    if (
      norm === 'Cargo.toml' ||
      norm === 'Cargo.lock' ||
      norm === 'package.json' ||
      norm === 'package-lock.json' ||
      norm.startsWith('.github/')
    ) {
      plan.description.push(`Critical build/manifest change: ${norm}`);
      plan.frontendUnit = true;
      plan.frontendA11y = true;
      plan.rustFull = true;
      plan.rustClippy = true;
      plan.rustFmt = true;
      return plan;
    }

    // Frontend JS / HTML / CSS changes
    if (
      norm.startsWith('static/') ||
      norm.startsWith('tests/frontend') ||
      norm.startsWith('tests/accessibility')
    ) {
      plan.frontendUnit = true;
      plan.frontendA11y = true;
      plan.description.push(`Frontend change: ${norm}`);
    }

    // E2E Playwright changes
    if (norm.startsWith('tests/e2e/')) {
      plan.e2eSpecs.add(norm);
      plan.description.push(`E2E spec change: ${norm}`);
    }

    // Rust changes
    if (norm.endsWith('.rs')) {
      hasRustChanges = true;
      plan.rustFmt = true;
      plan.rustClippy = true;

      if (norm === 'src/main.rs' || norm === 'src/lib.rs') {
        plan.rustFull = true;
      } else if (norm.startsWith('src/db/')) {
        plan.rustModules.add('db::');
      } else if (norm.startsWith('src/routes/')) {
        plan.rustModules.add('routes::');
      } else if (norm.startsWith('src/scraper')) {
        plan.rustModules.add('scraper::');
      } else if (norm.startsWith('src/security')) {
        plan.rustModules.add('security::');
      } else if (norm.startsWith('src/importer')) {
        plan.rustModules.add('importer::');
      } else if (norm.startsWith('src/geocoder')) {
        plan.rustModules.add('geocoder::');
      } else if (norm.startsWith('src/plus_code')) {
        plan.rustModules.add('plus_code::');
      } else if (norm.startsWith('src/models')) {
        plan.rustModules.add('models::');
      } else {
        plan.rustFull = true;
      }
      plan.description.push(`Rust change: ${norm}`);
    }
  }

  return plan;
}

function runCommand(cmd, label) {
  console.log(`\n\x1b[36m▶ [Affected Test Runner] ${label}\x1b[0m`);
  console.log(`\x1b[90m$ ${cmd}\x1b[0m`);

  if (isDryRun) {
    console.log(`\x1b[33m(Dry Run) Skipped execution\x1b[0m`);
    return true;
  }

  const result = spawnSync(cmd, { shell: true, stdio: 'inherit' });
  if (result.status !== 0) {
    console.error(`\n\x1b[31m✖ ${label} FAILED with exit code ${result.status}\x1b[0m\n`);
    return false;
  }
  return true;
}

function main() {
  console.log('\x1b[1m\x1b[35m=== ⚡ bList Affected Test Matrix ===\x1b[0m');

  const changed = getChangedFiles();
  const plan = resolveAffectedTests(changed);

  if (changed && changed.length > 0) {
    console.log(`\x1b[90mFound ${changed.length} modified file(s):\x1b[0m`);
    changed.forEach((f) => console.log(`  • ${f}`));
  }

  let passed = true;

  // 1. Rust Formatting Check
  if (plan.rustFmt) {
    passed = passed && runCommand('cargo fmt --all -- --check', 'Rust Code Formatting Check');
    if (!passed) process.exit(1);
  }

  // 2. Frontend Unit & A11y Tests
  if (plan.frontendUnit || plan.frontendA11y) {
    const testFiles = [];
    if (plan.frontendUnit) testFiles.push('tests/frontend.test.js');
    if (plan.frontendA11y) testFiles.push('tests/accessibility.test.js');
    passed = passed && runCommand(`node --test ${testFiles.join(' ')}`, 'Frontend Unit & A11y Tests');
    if (!passed) process.exit(1);
  }

  // 3. Rust Backend Tests (Full or Modular)
  if (plan.rustFull) {
    passed = passed && runCommand('cargo test', 'Rust Full Backend Test Suite');
    if (!passed) process.exit(1);
  } else if (plan.rustModules.size > 0) {
    const filter = Array.from(plan.rustModules).join(' ');
    passed = passed && runCommand(`cargo test ${filter}`, `Targeted Rust Tests (${Array.from(plan.rustModules).join(', ')})`);
    if (!passed) process.exit(1);
  }

  // 4. Rust Linter (Clippy)
  if (plan.rustClippy) {
    passed = passed && runCommand('cargo clippy --all-targets', 'Rust Clippy Linter Check');
    if (!passed) process.exit(1);
  }

  // 5. Targeted E2E Playwright Specs
  if (plan.e2eSpecs.size > 0) {
    const specs = Array.from(plan.e2eSpecs).join(' ');
    passed = passed && runCommand(`npx playwright test ${specs}`, `Targeted Playwright E2E (${specs})`);
    if (!passed) process.exit(1);
  }

  console.log('\n\x1b[32m✔ All affected tests and checks passed cleanly! Safe to commit & push.\x1b[0m\n');
}

main();
