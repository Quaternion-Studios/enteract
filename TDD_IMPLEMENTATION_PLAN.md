# Test-Driven Development (TDD) Implementation Plan
## Enteract Project

**Prepared by:** Evan (enteract/crew/evan)
**Date:** 2026-01-25
**Status:** Draft for Review

---

## Executive Summary

This plan establishes comprehensive TDD practices for the Enteract project, covering both Rust backend and Vue frontend. Current state shows partial test coverage (78 frontend tests, 38 Rust tests) with some failing tests that need fixing. The plan prioritizes critical paths, defines clear coverage targets, and outlines a phased implementation approach suitable for parallel execution via polecats.

---

## 1. Current Test State Analysis

### 1.1 Frontend Tests (TypeScript/Vue)
**Framework:** Vitest + Vue Test Utils + Testing Library
**Location:** `src/tests/`
**Current Status:**
- ✅ 60 tests passing
- ❌ 18 tests failing
- **Test Files:**
  - `src/tests/composables/useSpeechTranscription.test.ts` (12 tests, passing)
  - `src/tests/composables/useWindowManager.test.ts` (7 tests, passing)
  - `src/tests/components/ControlPanel.test.ts` (8 tests, passing)
  - `src/tests/components/ControlPanelButtons.test.ts` (12 tests, passing)
  - `src/tests/components/WindowInteractions.test.ts` (4 tests, passing)
  - `src/tests/components/ChatWindow.test.ts` (18 tests, failing - mock issues)
  - `src/tests/components/ConversationalWindow.test.ts` (17 tests, failing - mock issues)

**Key Issues:**
- Missing `getMessagePersistenceStatus()` in conversation store mock
- Several components lack test coverage entirely
- No E2E tests

### 1.2 Rust Tests
**Framework:** Native Rust `#[test]` + `#[cfg(test)]` modules
**Location:** Inline + `src-tauri/src/audio_loopback/tests/`
**Current Status:**
- ✅ 34 tests passing
- ❌ 4 tests failing
- **Failing Tests:**
  - `audio_loopback::shared::vad::tests::test_silence_detection`
  - `audio_loopback::shared::vad::tests::test_zero_crossing_rate`
  - `audio_loopback::shared::vad::tests::test_speech_segments`
  - `chunking_service::tests::test_sentence_splitting`

**Test Distribution:**
- `audio_loopback/` - 10+ tests (lifecycle tests marked #[ignore], device/engine tests passing)
- `data/conversation/` - 15 tests (all passing)
- `chunking_service` - 4 tests (1 failing)
- `search_service` - 2 tests (passing)
- `simple_embedding_service` - 4 tests (passing)

**Modules WITHOUT Tests:**
- `transparency.rs` - Critical for UI (0 tests)
- `window_manager.rs` - Window positioning (0 tests)
- `eye_tracking.rs` - ML tracking (0 tests)
- `speech.rs` - Whisper integration (0 tests)
- `ollama.rs` - AI responses (0 tests)
- `screenshot.rs` - Screen capture (0 tests)
- `file_handler.rs` - File uploads (0 tests)
- `mcp/` module - Multi-command processing (0 tests)
- `rag_system.rs`, `enhanced_rag_system.rs` - RAG (0 tests)

### 1.3 CI/CD
**Current Status:** ❌ No CI/CD pipeline configured
- No GitHub Actions workflows
- No automated test runs on PR/commit
- No coverage reporting

---

## 2. Testing Framework Strategy

### 2.1 Frontend Testing Stack
**Primary Framework:** Vitest (already configured)
- ✅ Fast, Vite-native
- ✅ Compatible with Vue Test Utils
- ✅ Coverage reporting built-in
- ✅ UI mode for debugging

**Supporting Tools:**
- `@vue/test-utils` - Component testing
- `@testing-library/vue` - User-centric testing
- `happy-dom` / `jsdom` - DOM simulation
- `@vitest/ui` - Test UI (already installed)

**Additional Tools to Add:**
- `@vitest/coverage-v8` - Coverage reporting (add to devDeps)
- `playwright` or `cypress` - E2E testing (Phase 3)

### 2.2 Rust Testing Stack
**Primary Framework:** Native Rust test framework
- ✅ Built into cargo
- ✅ Good parallelization
- ✅ Clear assertions

**Supporting Tools:**
- `mockall` - Mocking (not yet installed)
- `proptest` - Property-based testing (not yet installed)
- `criterion` - Benchmarking (optional)
- `tarpaulin` - Code coverage (install as dev tool)

**Tauri-Specific:**
- Mock `tauri::Window` for command tests
- Test invoke handlers without full Tauri runtime

---

## 3. Directory Structure

### 3.1 Frontend Tests
```
src/
├── components/
│   └── *.vue              # Component source
├── composables/
│   └── *.ts               # Composable source
├── tests/
│   ├── __mocks__/         # Shared mocks (Tauri, stores)
│   ├── __fixtures__/      # Test data
│   ├── components/
│   │   └── *.test.ts      # Component tests
│   ├── composables/
│   │   └── *.test.ts      # Composable tests
│   ├── stores/
│   │   └── *.test.ts      # Pinia store tests
│   ├── integration/       # Integration tests (NEW)
│   └── e2e/               # E2E tests (NEW, Phase 3)
```

### 3.2 Rust Tests
```
src-tauri/src/
├── module.rs              # Module source
├── module/
│   ├── mod.rs
│   └── tests/             # Module-specific test directory
│       └── feature_tests.rs
└── tests/                 # Integration tests (NEW)
    ├── commands_test.rs   # Tauri command tests
    └── helpers/           # Test utilities
```

---

## 4. Coverage Targets

### 4.1 Phase 1 Targets (Critical Paths)
- **Core Commands:** 80%+ coverage
  - `transparency.rs` commands
  - `window_manager.rs` commands
  - `speech.rs` commands
- **UI Components:** 70%+ coverage
  - ControlPanel, ChatWindow, ConversationalWindow
- **Composables:** 80%+ coverage
  - useTransparentClickthrough, useWindowManager, useSpeechTranscription

### 4.2 Phase 2 Targets (Full Coverage)
- **All Rust modules:** 75%+ coverage
- **All Vue components:** 70%+ coverage
- **All composables:** 80%+ coverage
- **Stores:** 85%+ coverage

### 4.3 Phase 3 Targets (Advanced)
- **E2E tests:** Key user flows covered
- **Visual regression:** Component snapshots
- **Performance benchmarks:** Critical paths < target latency

---

## 5. Priority Order (What to Test First)

### Phase 1: Critical Paths & Failing Tests (IMMEDIATE)
**Duration:** 1-2 weeks
**Goal:** Fix failing tests, cover critical user-facing features

**Priority 1A: Fix Failing Tests**
1. Fix ConversationalWindow mock issues (18 tests)
2. Fix ChatWindow mock issues (missing store methods)
3. Fix VAD audio tests (silence detection, zero crossing)
4. Fix chunking_service sentence splitting test

**Priority 1B: Cover Untested Critical Modules**
1. `transparency.rs` - Window transparency (NEW)
   - Test `set_mouse_passthrough()`
   - Test `start_mouse_tracking()`
   - Test coordinate conversion
2. `window_manager.rs` - Window positioning
   - Test position/size getters
   - Test screen bounds
3. `speech.rs` - Whisper integration
   - Test model initialization
   - Test transcription commands
   - Mock Whisper for fast tests

**Priority 1C: Frontend Critical Components**
1. ChatSidebarAdapter (not tested)
2. MessageList (not tested)
3. ConversationalAI (not tested)
4. useTransparentClickthrough (just implemented, 0 tests)

### Phase 2: Comprehensive Coverage (4-6 weeks)
**Goal:** Achieve 75%+ coverage across codebase

**Rust Modules:**
1. `ollama.rs` - AI response generation (complex, many commands)
2. `mcp/` - Multi-command processing module
3. `rag_system.rs` + `enhanced_rag_system.rs` - RAG systems
4. `eye_tracking.rs` - ML gaze tracking
5. `screenshot.rs` - Screen capture
6. `file_handler.rs` - File upload/validation
7. `data/` module - Database operations (partial coverage, expand)

**Frontend:**
1. All remaining components
2. All remaining composables
3. Pinia stores (app.ts, conversation.ts)
4. Utility functions (markdownRenderer, etc.)

### Phase 3: Advanced Testing (4-8 weeks)
**Goal:** E2E tests, visual regression, performance

1. **E2E Tests** (Playwright or Cypress)
   - User opens app
   - User starts voice recording
   - User sends chat message
   - User adjusts window transparency
   - User toggles AI models panel
2. **Visual Regression** (Playwright snapshots)
   - Component screenshots for all states
   - Catch unintended UI changes
3. **Performance Benchmarks** (Criterion for Rust)
   - RAG search latency
   - Embedding generation time
   - Audio processing latency

---

## 6. Polecat Parallelization Strategy

### 6.1 Task Distribution Model
Polecats are ideal for parallel test writing. Structure:

**Task Template:**
```
Title: Write tests for [module]
Description:
- Module: src-tauri/src/[module].rs
- Coverage target: 80%
- Test scenarios: [list]
- Mock requirements: [list]
- Reference: [similar tested module]
```

**Example Distribution (Phase 1B):**
```
Polecat Alpha → transparency.rs tests
Polecat Bravo → window_manager.rs tests
Polecat Charlie → speech.rs tests
Polecat Delta → ChatSidebarAdapter tests
Polecat Echo → useTransparentClickthrough tests
```

### 6.2 Coordination
- Each polecat gets isolated module
- No shared files to avoid merge conflicts
- Code review after completion
- Merge after CI passes

### 6.3 Test Template Bead
Create a "test template" bead with:
- Standard test structure
- Mocking patterns
- Assertion helpers
- Coverage thresholds

---

## 7. CI/CD Integration Strategy

### 7.1 GitHub Actions Workflow
Create `.github/workflows/test.yml`:

```yaml
name: Tests

on:
  push:
    branches: [main, dev, crew/*]
  pull_request:
    branches: [main, dev]

jobs:
  frontend-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm test -- --run
      - run: npm run test:coverage
      - uses: codecov/codecov-action@v3
        with:
          files: ./coverage/coverage-final.json

  rust-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cd src-tauri && cargo test --lib
      - run: cd src-tauri && cargo tarpaulin --out Xml
      - uses: codecov/codecov-action@v3
        with:
          files: ./src-tauri/cobertura.xml

  build-verification:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run tauri build -- --debug
```

### 7.2 Pre-commit Hooks
Install `husky` + `lint-staged`:
```bash
npm install -D husky lint-staged
npx husky install
```

Create `.husky/pre-commit`:
```bash
#!/bin/sh
npm test -- --run --passWithNoTests
cd src-tauri && cargo test --lib
```

### 7.3 Coverage Reporting
- Use Codecov or Coveralls
- Fail PR if coverage drops > 2%
- Display badges in README

---

## 8. GUI/Visual Regression Testing (Stretch Goal)

### 8.1 Playwright for E2E + Visual Regression
**Why Playwright:**
- Native Tauri support via webdriver
- Built-in screenshot comparison
- Fast, reliable, cross-browser

**Setup:**
```bash
npm install -D @playwright/test
npx playwright install
```

**Example Test:**
```typescript
import { test, expect } from '@playwright/test'

test('control panel renders correctly', async ({ page }) => {
  await page.goto('http://localhost:1420')

  // Visual regression
  await expect(page).toHaveScreenshot('control-panel.png')

  // Interaction test
  await page.click('[data-testid="chat-button"]')
  await expect(page.locator('.chat-window')).toBeVisible()
})
```

### 8.2 Component Snapshot Testing
Use Vitest's built-in snapshot testing:
```typescript
import { mount } from '@vue/test-utils'
import { expect, test } from 'vitest'

test('ControlPanel snapshot', () => {
  const wrapper = mount(ControlPanel, { props: { ... } })
  expect(wrapper.html()).toMatchSnapshot()
})
```

---

## 9. Implementation Phases & Estimates

### Phase 1: Critical Paths (1-2 weeks)
**Effort:** 40-60 hours
**Parallelizable:** Yes (5 polecats)
- Fix 22 failing tests
- Write tests for transparency.rs, window_manager.rs, speech.rs
- Write tests for useTransparentClickthrough, ChatSidebarAdapter
- Add basic CI workflow

### Phase 2: Comprehensive Coverage (4-6 weeks)
**Effort:** 120-180 hours
**Parallelizable:** Yes (10+ polecats)
- Test all remaining Rust modules
- Test all remaining frontend components/composables
- Achieve 75%+ coverage
- Add coverage reporting

### Phase 3: Advanced Testing (4-8 weeks)
**Effort:** 100-200 hours
**Parallelizable:** Partially (coordination needed for E2E)
- E2E test suite (20+ tests)
- Visual regression (50+ snapshots)
- Performance benchmarks (10+ benchmarks)
- Documentation and maintenance guide

**Total Estimated Effort:** 260-440 hours (depends on parallelization)

---

## 10. Success Metrics

### Quantitative Metrics
- ✅ 90%+ tests passing (currently 77%)
- ✅ 75%+ code coverage (Rust)
- ✅ 70%+ code coverage (Frontend)
- ✅ CI passing on all PRs
- ✅ < 5min CI execution time

### Qualitative Metrics
- Confidence in refactoring (no fear of breaking changes)
- Faster bug detection (catch issues before production)
- Better documentation (tests as examples)
- Easier onboarding (tests show how code works)

---

## 11. Risks & Mitigation

### Risk 1: Flaky Tests
**Mitigation:**
- Avoid timing-dependent tests
- Use deterministic mocks
- Retry flaky tests 3x before failing

### Risk 2: Slow Tests
**Mitigation:**
- Parallelize Rust tests (cargo test already does this)
- Use `happy-dom` instead of `jsdom` (faster)
- Mock expensive operations (Whisper, Ollama)

### Risk 3: Coverage Obsession
**Mitigation:**
- Focus on critical paths first
- Don't test getters/setters for coverage %
- Test behavior, not implementation

### Risk 4: Outdated Tests
**Mitigation:**
- Run tests in CI on every commit
- Fail PRs if tests don't pass
- Regular test review/refactoring

---

## 12. Next Steps (After Plan Approval)

1. **Create implementation beads** for each phase
2. **Set up CI/CD workflow** (`.github/workflows/test.yml`)
3. **Fix failing tests** (Priority 1A)
4. **Assign polecats** to Phase 1B critical modules
5. **Weekly progress review** with Mayor
6. **Coverage dashboard** setup (Codecov)

---

## 13. Questions for Mayor

1. **Priority confirmation:** Does Phase 1 priority order match your expectations?
2. **Coverage targets:** Are 75% Rust / 70% Frontend acceptable, or aim higher?
3. **E2E tooling:** Playwright vs Cypress preference?
4. **Timeline:** Is 2-week Phase 1 aggressive enough, or should we accelerate?
5. **Polecat availability:** How many polecats can be allocated to Phase 1B?

---

## Appendix A: Test Command Reference

### Frontend Tests
```bash
npm test                  # Run tests in watch mode
npm test -- --run         # Run tests once
npm run test:ui           # Open Vitest UI
npm run test:coverage     # Generate coverage report
```

### Rust Tests
```bash
cargo test                # Run all tests
cargo test --lib          # Run library tests only
cargo test -- --ignored   # Run ignored tests
cargo tarpaulin           # Coverage report (requires tarpaulin)
```

### Combined
```bash
npm run tauri test        # Run both frontend + backend (if configured)
```

---

**End of Plan**
