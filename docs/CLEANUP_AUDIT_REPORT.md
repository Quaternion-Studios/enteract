# Codebase Cleanup Audit Report

**Auditor**: enteract/crew/nic
**Date**: 2026-01-29
**Scope**: Full codebase review - docs, code smells, file structure

---

## Executive Summary

| Category | Critical | High | Medium | Low |
|----------|----------|------|--------|-----|
| Dead Code | 1 | 4 | 8 | - |
| Duplicated Logic | - | 4 | 2 | - |
| Stale Docs | - | - | 4 | - |
| TODOs/FIXMEs | - | 4 | 6 | 5 |
| Code Complexity | - | 2 | 5 | - |

**Estimated cleanup effort**: 2-3 focused sessions

---

## PRIORITY 1: CRITICAL - Delete Immediately

### 1.1 Duplicate Command Registration in lib.rs

**File**: `src-tauri/src/lib.rs`
**Lines**: 378-386 (duplicate of 315-325)

```rust
// DUPLICATE - These are registered twice:
save_conversation_message,
batch_save_conversation_messages,
update_conversation_message,
delete_conversation_message,
save_conversation_insight,
get_conversation_insights,
```

**Action**: Delete lines 378-386
**Risk**: None - exact duplicates
**Impact**: Prevents potential double-invocation bugs

---

## PRIORITY 2: HIGH - Delete Unused Code

### 2.1 Unused Vue Components (8 files, ~800 lines)

| File | Lines | Purpose | Delete? |
|------|-------|---------|---------|
| `src/components/core/AdvancedGazeDemo.vue` | ~100 | Demo component | YES |
| `src/components/core/AudioLoopbackControl.vue` | ~150 | Superseded UI | YES |
| `src/components/core/WindowHeader.vue` | ~50 | Unused header | YES |
| `src/components/core/RefractionBorder.vue` | ~80 | Visual experiment | YES |
| `src/components/core/MinimizedView.vue` | ~100 | Unused view | YES |
| `src/components/core/HomeScreen.vue` | ~150 | Replaced screen | VERIFY |
| `src/components/DatabaseStatus.vue` | ~80 | Debug component | YES |
| `src/components/MigrationHelper.vue` | ~100 | One-time use | YES |

**Action**: Delete after verifying no dynamic imports
**Verification**: `grep -r "AdvancedGazeDemo\|AudioLoopbackControl\|WindowHeader" src/`

### 2.2 Unused Composables (4 files, ~650 lines)

| File | Lines | Why Unused |
|------|-------|------------|
| `src/composables/useConversationTempo.ts` | 223 | Experimental, never imported |
| `src/composables/useGazeWindowControl.ts` | ~100 | Superseded by other gaze code |
| `src/composables/useGPUStatus.ts` | 79 | Never imported |
| `src/composables/useResponseGenerator.ts` | 269 | References non-existent Tauri command |

**Action**: Delete all 4 files
**Also**: Remove exports from `src/composables/index.ts`

### 2.3 Commented-Out Code Block

**File**: `src/components/core/ChatWindow.vue`
**Lines**: 345-349

```typescript
// Window resizing composable - not currently used in this component
// const {
//   chatWindowSize,
//   isResizing,
//   startResize
// } = useWindowResizing()
```

**Action**: Delete these 5 lines

### 2.4 Unused Rust Function

**File**: `src-tauri/src/ollama.rs`
**Line**: 1081

```rust
pub async fn generate_with_custom_timeouts(...)
```

**Status**: Defined as Tauri command but NOT registered in invoke_handler
**Action**: Either register or delete

---

## PRIORITY 3: HIGH - Consolidate Duplicated Code

### 3.1 RAG System Duplication (SEVERE)

**Files**:
- `src-tauri/src/rag_commands.rs` (161 lines, 8 commands)
- `src-tauri/src/enhanced_rag_commands.rs` (356 lines, 15 commands)
- `src-tauri/src/rag_system.rs` (500+ lines)
- `src-tauri/src/enhanced_rag_system.rs` (1000+ lines)

**Problem**: Two parallel implementations with `enhanced_` prefix doing the same thing.

**Duplicated patterns**:
- `upload_document` vs `upload_enhanced_document`
- `delete_document` vs `delete_enhanced_document`
- `search_documents` vs `search_enhanced_documents`
- `get_rag_settings` vs `get_enhanced_rag_settings`

**Action**: Consolidate into single implementation. Keep `enhanced_*` version, delete original.

### 3.2 State Lock Pattern (9 duplicates)

**File**: `src-tauri/src/enhanced_rag_commands.rs`
**Lines**: 42-48, 75-81, 96-102, 114-120, 131-137, 228-234, 265-271, 282-288, 300-306

```rust
// This exact block appears 9 times:
let system = {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    match &*rag_state {
        Some(sys) => Ok(sys.clone()),
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}?;
```

**Action**: Extract to helper function:
```rust
fn get_rag_system(state: &EnhancedRagSystemState) -> Result<EnhancedRagSystem, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    rag_state.clone().ok_or_else(|| "Enhanced RAG system not initialized".to_string())
}
```

### 3.3 ConversationStorage Initialization (12 duplicates)

**File**: `src-tauri/src/data/conversation/commands.rs`
**Lines**: 14-18, 22-27, 31-39, 43-48, 75-95, 106-126, 136-140, 149-153, 163-167, 175-179, 191-198

```rust
// This pattern appears 12 times:
match ConversationStorage::new(&app_handle) {
    Ok(mut storage) => // operation
    Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
}
```

**Action**: Extract initialization or use a macro

### 3.4 SQLite Initialization Duplication

**Files**:
- `src-tauri/src/data/conversation/storage.rs` (lines 34-60)
- `src-tauri/src/data/chat/storage.rs` (lines 32-56)

**Problem**: Identical PRAGMA configuration, WAL mode setup, error handling

**Action**: Extract to `src-tauri/src/data/connection_pool.rs` (file exists but not used for this)

---

## PRIORITY 4: MEDIUM - Stale Documentation

### 4.1 Docs to Consolidate/Remove

| File | Status | Action |
|------|--------|--------|
| `resources/context-chat.md` | STALE | Merge into PHASE_2_COMPLETION.md |
| `resources/chat-context-memory-management.md` | STALE | Merge or delete (duplicate content) |
| `resources/whisper-rs-branch.md` | OUTDATED | Verify if branch strategy still applies |
| `resources/transparency.md` | STALE | Delete - verbose duplicate of TRANSPARENCY_USAGE.md |

**Note**: Core documentation is excellent and current (84% of docs are up-to-date)

---

## PRIORITY 5: MEDIUM - Resolve Stale TODOs

### 5.1 High Priority TODOs (Core Features)

| File:Line | TODO | Action |
|-----------|------|--------|
| `rag_system.rs:267` | Implement PDF text extraction | Create issue or implement |
| `rag_system.rs:271` | Implement OCR for images | Create issue or remove stub |
| `mcp/server.rs:368` | Replace with actual LLM call | Core MCP feature - implement |
| `enhanced_rag_system.rs:379` | Implement OCR | Duplicate of above - remove |

### 5.2 Medium Priority TODOs

| File:Line | TODO | Action |
|-----------|------|--------|
| `rag_system.rs:332` | Background embedding generation | Create issue |
| `rag_commands.rs:150` | Local embedding model | Related to storage research |
| `rag_commands.rs:159` | Cache clearing | Implement or remove |
| `mcp/commands.rs:216` | Step-by-step execution | Create issue |
| `enhancedRagService.ts:463` | Migration logic | Clarify if needed |
| `enhancedRagService.ts:481` | Actual metrics | Implement or remove |

### 5.3 Keep (Test Scaffolding)

These TODOs in `audio_loopback/tests/` are intentional placeholders for Phase 1:
- `sample_rate_tests.rs:13`
- `device_enumeration_tests.rs:13`
- `capture_lifecycle_tests.rs:16`

---

## PRIORITY 6: MEDIUM - Code Complexity

### 6.1 Long Functions to Refactor

| File | Function | Lines | Issue |
|------|----------|-------|-------|
| `data/conversation/storage.rs` | `merge_partial_messages` | 115 | Complex merging logic |
| `data/conversation/storage.rs` | `load_conversations` | 100+ | Multi-step transformation |
| `enhanced_rag_commands.rs` | `validate_enhanced_file_upload` | 42 | Nested validation |
| `enhanced_rag_commands.rs` | `get_embedding_status` | 28 | Multiple HashMap ops |
| `ollama.rs` | Multiple streaming functions | 100+ | Complex async handling |

### 6.2 Deep Nesting (4+ levels)

**File**: `src-tauri/src/enhanced_rag_commands.rs`
- Lines 191-218: `get_embedding_status` - 4 levels
- Lines 314-356: `validate_enhanced_file_upload` - 4 levels

**Action**: Extract inner logic to helper functions

---

## PRIORITY 7: LOW - Code Style Issues

### 7.1 println! Instead of Logging

**Files affected**:
- `data/conversation/storage.rs` (10+ occurrences with emoji)
- `data/chat/storage.rs` (5+ occurrences)

**Pattern**:
```rust
println!("✅ WAL mode enabled successfully");
println!("⚠️ Warning: Could not set journal mode: {}", e);
```

**Action**: Replace with proper `tracing` or `log` crate

### 7.2 Inconsistent Error Handling

4 different patterns across codebase:
1. `.map_err(|e| e.to_string())`
2. `Err(format!("Failed to...: {}", e))`
3. `match ... Err(e) => Err(format!(...))`
4. `.map_err(|e| { println!(...); e })?`

**Action**: Standardize on single pattern (recommend `thiserror` crate)

### 7.3 Type Safety Issues

**File**: `src/composables/useResponseGenerator.ts`
```typescript
tempo: any  // Should have proper type
```

---

## FILE STRUCTURE OBSERVATIONS

### Good Organization
- `src-tauri/src/data/` - Well-structured with chat/conversation separation
- `src-tauri/src/audio_loopback/` - Clean trait-based platform abstraction
- `src-tauri/src/mcp/` - Properly modularized

### Issues
1. **19 files in src-tauri/src/ root** - Consider grouping:
   - `rag_*.rs`, `*_rag_*.rs` → `src-tauri/src/rag/`
   - `*_service.rs` → `src-tauri/src/services/`

2. **Parallel RAG implementations** - `rag_*` and `enhanced_rag_*` should be unified

---

## CLEANUP CHECKLIST

### Session 1: Dead Code Removal
- [ ] Delete duplicate command registration in `lib.rs:378-386`
- [ ] Delete 8 unused Vue components
- [ ] Delete 4 unused composables
- [ ] Update `src/composables/index.ts` exports
- [ ] Delete commented code in `ChatWindow.vue:345-349`
- [ ] Decide on `ollama.rs:generate_with_custom_timeouts`

### Session 2: Consolidation
- [ ] Merge `rag_system.rs` into `enhanced_rag_system.rs`
- [ ] Merge `rag_commands.rs` into `enhanced_rag_commands.rs`
- [ ] Extract state lock helper in `enhanced_rag_commands.rs`
- [ ] Extract storage init helper in `data/conversation/commands.rs`
- [ ] Unify SQLite initialization in `connection_pool.rs`

### Session 3: Documentation & TODOs
- [ ] Delete `resources/transparency.md`
- [ ] Consolidate context/chat docs
- [ ] Review `whisper-rs-branch.md` relevance
- [ ] Create issues for unresolved TODOs
- [ ] Remove stale/duplicate TODOs

---

## ESTIMATED IMPACT

| Metric | Before | After |
|--------|--------|-------|
| Unused Vue components | 8 | 0 |
| Unused composables | 4 | 0 |
| Duplicate code blocks | 25+ | <5 |
| Stale docs | 4 | 0 |
| Untracked TODOs | 15 | 0 (converted to issues) |
| Lines removed | - | ~1,500-2,000 |

---

*Report generated by enteract/crew/nic*
