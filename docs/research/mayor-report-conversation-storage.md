# Research Report: Conversation Storage Architecture

**To**: Mayor
**From**: enteract/crew/nic
**Date**: 2026-01-29
**Re**: Efficient storage for personal AI conversation history

---

## Assignment

Research efficient storage for personal AI with full conversation/meeting history, citation capability, semantic search, on resource-constrained devices.

## Executive Summary

Evaluated three approaches for local vector storage: **sqlite-vector**, **Qdrant**, and **LEANN**.

**Recommendation: sqlite-vector with int8 quantization** for Enteract's use case.

---

## Options Evaluated

### 1. sqlite-vector (SQLite.ai) — RECOMMENDED

**What it is**: SQLite extension storing vectors as BLOBs in regular tables with SIMD-optimized search.

| Aspect | Detail |
|--------|--------|
| Memory | 30MB cap during queries |
| Compression | 4x (int8 quantization) |
| Quality retention | 98-100% |
| Query speed | ~4ms with quantization + preload |
| Integration | C implementation, trivial Rust FFI |
| Storage | 100K messages × 384 dims × 1 byte = ~37MB |

**Pros**:
- Single database file (enteract.db) - simpler ops
- No separate server process
- Mature SQLite ecosystem

**Cons**:
- Newer project, smaller community than Qdrant
- No HNSW index yet (brute-force with quantization)

### 2. Qdrant

**What it is**: Full-featured vector database written in Rust.

**Pros**: Production-grade, excellent Rust integration, built-in quantization

**Cons**: Requires separate server process, designed for multi-tenant/distributed use

**Verdict**: Overkill for single-user local app.

### 3. LEANN (github.com/yichuan-w/LEANN)

**What it is**: Lightweight vector DB that recomputes embeddings on-demand instead of storing them. Achieves 97% storage reduction (60M chunks in 6GB vs 201GB).

**Pros**:
- Dramatic storage savings (33x compression vs 4x for int8)
- Designed specifically for personal AI use case
- Graph-based (HNSW/DiskANN backends)

**Cons**:
- Python-based (requires subprocess/sidecar for Tauri/Rust)
- Slower queries (recomputes embeddings at search time ~1.5s for 100 candidates)
- Newer/research-stage project

**Analysis**: LEANN's recompute-on-demand trades query latency for storage. The 97% storage savings matter at 60M+ scale, but personal conversation history (<1M messages over years) fits comfortably with int8 quantization (~400MB).

**Verdict**: Interesting for extreme scale, but Python integration complexity and query latency tradeoff not worth it for Enteract's scale.

---

## Recommended Architecture

```
┌─────────────────────────────────────────────┐
│              enteract.db (SQLite)           │
├─────────────────────────────────────────────┤
│  Existing: chat_messages, conversation_*    │
├─────────────────────────────────────────────┤
│  NEW: message_embeddings (int8 BLOB)        │
│       via sqlite-vector extension           │
└─────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
┌─────────────────┐   ┌─────────────────┐
│  Tantivy (BM25) │   │  sqlite-vector  │
│  keyword search │   │  semantic search│
└────────┬────────┘   └────────┬────────┘
         └──────────┬──────────┘
                    ▼
            ┌──────────────┐
            │  RRF Fusion  │
            │  (k=60)      │
            └──────────────┘
```

**Embedding model**: all-MiniLM-L6-v2
- Size: 22MB
- Dimensions: 384
- Inference: Local CPU, ~15ms per text

**Memory budget** (512MB total for AI features):

| Component | Allocation |
|-----------|------------|
| Embedding model | 50MB |
| sqlite-vector | 30MB |
| Tantivy BM25 | 20MB |
| Buffers/cache | 60MB |
| **Headroom** | 352MB (Whisper, LLM context) |

---

## Current Gaps in Enteract

1. **SimpleEmbeddingService is placeholder** — generates deterministic hashes, not ML embeddings
2. **No vector index** — brute-force scan for similarity
3. **Conversation messages not indexed** — stored but not semantically searchable
4. **No citation extraction** — noted as nice-to-have, not critical per overseer

---

## Implementation Priorities

| Phase | Work |
|-------|------|
| 1 | Replace SimpleEmbeddingService with MiniLM-L6-v2, add sqlite-vector extension |
| 2 | Implement RRF fusion, index existing conversations |
| 3 | Background embedding queue, cross-session context |

---

## Deliverables

- **Research doc**: `docs/research/conversation-storage-architecture.md`
- **Branch**: `research/conversation-storage`
- **PR**: https://github.com/Quaternion-Studios/enteract/pull/new/research/conversation-storage

---

## Open Question

**Primary constraint to optimize for**: storage size, query latency, or integration simplicity?

Current recommendation optimizes for **integration simplicity + query latency**, accepting moderate storage (int8 = 4x compression). If storage becomes critical concern, LEANN hybrid approach possible but adds Python dependency.

---

*—nic, enteract/crew*
