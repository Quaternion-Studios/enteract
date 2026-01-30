# Conversation Data Storage Architecture Research

> **Vision**: Personal AI that stores full conversation/meeting history, can cite specific parts, runs locally on devices with limited resources.

## Executive Summary

This research evaluates storage architectures for a local personal AI that must:
- Store complete conversation and meeting history (text + audio transcripts)
- Enable fast semantic search with citation of specific parts
- Run efficiently on resource-constrained devices (laptops, tablets)
- Support real-time incremental indexing as conversations happen

**Recommended approach**: Hybrid SQLite + sqlite-vector with int8 quantization, BM25/vector fusion via RRF, and MiniLM-L6-v2 embeddings.

---

## Current Enteract Architecture

### What Exists

| Component | Implementation | Status |
|-----------|---------------|--------|
| **Structured Storage** | SQLite with WAL mode | Production-ready |
| **Full-text Search** | Tantivy (BM25) | Working |
| **Embedding Service** | Placeholder (text-feature hash) | Needs replacement |
| **Vector Search** | Brute-force in-memory | Not scalable |
| **Hybrid Search** | BM25 + vector blend | Partially implemented |

### Data Models

**Chat Sessions** (text-based Claude conversations):
- `chat_sessions`, `chat_messages`, `message_attachments`
- `thinking_processes`, `thinking_steps`, `message_metadata`

**Conversation Sessions** (audio/meeting transcripts):
- `conversation_sessions`, `conversation_messages`
- `conversation_insights`, `conversation_state`
- Includes: confidence scores, speaker_id, source (mic/loopback), timestamps

### Current Gaps

1. **No real embeddings**: `SimpleEmbeddingService` generates deterministic hashes, not semantic embeddings
2. **No vector index**: Cosine similarity computed via brute-force scan
3. **Conversations not indexed**: Audio transcripts stored but not searchable semantically
4. **No citation support**: Cannot reference specific message/timestamp ranges

---

## Storage Options Evaluated

### 1. SQLite Vector Extensions

#### sqlite-vec (Alex Garcia)
- **Pros**: Pure C, zero dependencies, runs anywhere (WASM, RPi), Mozilla-sponsored
- **Cons**: Brute-force KNN, no HNSW index yet
- **Memory**: Moderate - vectors in virtual tables
- **Links**: [GitHub](https://github.com/asg017/sqlite-vec), [Hybrid Search Guide](https://alexgarcia.xyz/blog/2024/sqlite-vec-hybrid-search/index.html)

#### sqlite-vector (SQLite.ai) - **RECOMMENDED**
- **Pros**: 30MB default memory, SIMD-optimized, int8/binary quantization, no virtual tables needed
- **Cons**: Newer project, less community adoption
- **Performance** (100K vectors, 384-dim):
  - Insert: 563ms (50% faster than sqlite-vec)
  - Query: 56ms plain, **3.97ms with quantization + preload**
  - Recall: Perfect with quantization
- **Links**: [GitHub](https://github.com/sqliteai/sqlite-vector), [SQLite.ai](https://www.sqlite.ai/sqlite-vector)

#### Verdict
**sqlite-vector** wins for resource-constrained use. 30MB memory cap, 17x faster queries with quantization, stores vectors as BLOBs in regular tables (simpler schema).

### 2. Dedicated Vector Databases

| Database | Memory | Local/Embedded | Verdict |
|----------|--------|----------------|---------|
| **Chroma** | Moderate | Yes, Python | Good for prototypes, not Rust |
| **Qdrant** | Low (w/ compression) | Yes, Rust | Overkill for single-user local |
| **LanceDB** | Low | Yes, Rust | Good option, less mature |
| **FAISS** | High | Library | Not a database, just indexing |

#### Verdict
Dedicated vector DBs add complexity. SQLite extensions keep everything in one database, simpler deployment.

### 3. Hybrid Approaches

**Best practice**: Combine BM25 keyword search + vector semantic search via Reciprocal Rank Fusion (RRF).

**Why hybrid matters**:
- Pure vector search misses exact matches (e.g., ticket ID "TS-01")
- Pure keyword search misses semantic similarity
- RRF combines rankings without raw score normalization issues

**RRF Formula**: `score = Σ 1/(k + rank)` where k=60 typically

**Implementation** (already partially in Enteract):
```rust
// Current config in search_service.rs
SearchConfig {
    bm25_weight: 0.7,
    vector_weight: 0.3,
    ...
}
```

**Recommendation**: Switch to RRF-based fusion instead of weighted score blending.

---

## Embedding Strategy

### Model Selection

| Model | Dimensions | Size | Speed (1K tokens) | Quality | Quantization-friendly |
|-------|------------|------|-------------------|---------|----------------------|
| **all-MiniLM-L6-v2** | 384 | 22MB | 14.7ms | Good | Yes |
| gte-small | 384 | 33MB | ~20ms | Better | Yes |
| bge-small-en-v1.5 | 384 | 33MB | ~20ms | Better | Yes |
| nomic-embed-text | 768 | 137MB | ~50ms | Best | Yes |

**Recommendation**: Start with **all-MiniLM-L6-v2** (22MB, fast, 384 dims - matches current config). Upgrade to gte-small if quality insufficient.

### Quantization Strategy

| Method | Memory Reduction | Quality Retention | Best For |
|--------|-----------------|-------------------|----------|
| Float32 (baseline) | 1x | 100% | Accuracy-critical |
| **Int8 (scalar)** | 4x | 98-100% | **Balanced (recommended)** |
| Binary (1-bit) | 24-32x | 80-95% | High-volume with rescoring |
| Binary + int8 rescore | ~40x | 95%+ | Production at scale |

**Recommendation**: Use **int8 quantization** for 4x memory reduction with near-perfect quality. sqlite-vector supports this natively.

### Storage Calculation

For 100,000 conversation messages:
- Float32: 100K × 384 × 4 bytes = **147 MB**
- Int8: 100K × 384 × 1 byte = **37 MB**
- Binary: 100K × 384 / 8 = **4.7 MB** (with rescoring buffer)

---

## Incremental Indexing Design

### Challenge
Conversations are append-only streams. Rebuilding the full index on every message is expensive.

### Approach: Write Buffer + Periodic Merge

```
New messages → Write buffer (in-memory, ~100 messages)
                    ↓ (flush trigger: count or time)
              Batch embed + append to index
                    ↓ (background)
              Merge into main index if needed
```

**Key principles** (from [CocoIndex](https://medium.com/@cocoindex.io/building-a-real-time-data-substrate-for-ai-agents-the-architecture-behind-cocoindex-729981f0f3a4)):
- Separate write and read paths
- Batch embedding generation (reduces model overhead)
- Append-only for conversation data (no updates needed)
- Query both main index + write buffer for real-time search

### Implementation Notes
- sqlite-vector supports direct INSERT without index rebuild
- Tantivy (current BM25) supports incremental commits
- Embed in batches of 50-100 messages to amortize model load

---

## Retrieval API Design

### Core Operations

```rust
/// Semantic search across all conversations
fn search_conversations(query: &str, options: SearchOptions) -> Vec<SearchResult>;

/// Citation: get specific message with context
fn get_message_with_context(message_id: &str, context_window: usize) -> MessageContext;

/// Time-range search
fn search_in_range(query: &str, start: DateTime, end: DateTime) -> Vec<SearchResult>;

/// Speaker-filtered search
fn search_by_speaker(query: &str, speaker_id: Option<&str>) -> Vec<SearchResult>;
```

### SearchResult Structure

```rust
struct SearchResult {
    message_id: String,
    session_id: String,
    content: String,
    timestamp: DateTime,
    speaker: Option<String>,
    source: MessageSource,  // Microphone, Loopback, Text

    // For citation
    context_before: Vec<Message>,
    context_after: Vec<Message>,

    // Scoring
    relevance_score: f32,
    bm25_score: f32,
    vector_score: f32,
}
```

### Citation Support

For "cite specific parts" requirement:

```rust
struct Citation {
    session_id: String,
    session_title: Option<String>,
    start_message_id: String,
    end_message_id: String,
    start_timestamp: DateTime,
    end_timestamp: DateTime,
    excerpt: String,  // The cited text
    formatted: String,  // "Meeting with Bob, Jan 15 2026, 2:34 PM - 2:36 PM"
}

fn create_citation(message_ids: Vec<String>) -> Citation;
fn format_citation(citation: &Citation, style: CitationStyle) -> String;
```

---

## Recommended Architecture

### Phase 1: Core Storage (Current + Improvements)

```
┌─────────────────────────────────────────────────────────┐
│                    enteract.db (SQLite)                 │
├─────────────────────────────────────────────────────────┤
│  chat_sessions, chat_messages, conversation_sessions,  │
│  conversation_messages, etc. (existing)                 │
├─────────────────────────────────────────────────────────┤
│  NEW: message_embeddings (int8 BLOB via sqlite-vector)  │
│  - message_id, embedding_int8, created_at               │
└─────────────────────────────────────────────────────────┘
```

### Phase 2: Search Infrastructure

```
┌─────────────────┐     ┌─────────────────┐
│  Tantivy Index  │     │  sqlite-vector  │
│  (BM25 FTS)     │     │  (KNN search)   │
└────────┬────────┘     └────────┬────────┘
         │                       │
         └───────────┬───────────┘
                     │
              ┌──────▼──────┐
              │  RRF Fusion │
              │  k=60       │
              └──────┬──────┘
                     │
              ┌──────▼──────┐
              │  Results    │
              │  + Citation │
              └─────────────┘
```

### Phase 3: Embedding Pipeline

```
┌──────────────┐     ┌───────────────┐     ┌──────────────┐
│ New Messages │────▶│ Write Buffer  │────▶│ Batch Embed  │
│ (real-time)  │     │ (in-memory)   │     │ (MiniLM)     │
└──────────────┘     └───────────────┘     └──────┬───────┘
                                                   │
                                                   ▼
                                           ┌──────────────┐
                                           │ Int8 Quant   │
                                           │ + Store      │
                                           └──────────────┘
```

---

## Memory Budget Analysis

Target: 512MB total for AI features on constrained device

| Component | Allocation | Notes |
|-----------|------------|-------|
| Embedding model | 50MB | MiniLM-L6-v2 loaded once |
| sqlite-vector | 30MB | Default memory cap |
| Tantivy reader | 20MB | BM25 index |
| Write buffer | 10MB | ~100 messages |
| Query cache | 50MB | Recent embeddings |
| **Headroom** | 352MB | For Whisper, LLM context, etc. |

With int8 quantization, 1 million messages requires ~384MB vector storage on disk, ~30MB in memory during search.

---

## Implementation Priorities

### Immediate (Phase 1)
1. Replace `SimpleEmbeddingService` with real MiniLM-L6-v2 inference
2. Add sqlite-vector extension to rusqlite build
3. Create `message_embeddings` table with int8 storage
4. Implement incremental embedding on new messages

### Short-term (Phase 2)
1. Implement RRF fusion replacing weighted blend
2. Add citation extraction from search results
3. Index existing conversation_messages
4. Add time-range and speaker filters

### Medium-term (Phase 3)
1. Background embedding queue with priority
2. Cross-session context (reference earlier conversations)
3. Summary generation for long sessions
4. Export/backup of conversation archive

---

## References

### SQLite Vector Extensions
- [sqlite-vec GitHub](https://github.com/asg017/sqlite-vec)
- [sqlite-vector GitHub](https://github.com/sqliteai/sqlite-vector)
- [State of Vector Search in SQLite](https://marcobambini.substack.com/p/the-state-of-vector-search-in-sqlite)

### Quantization
- [HuggingFace Embedding Quantization](https://huggingface.co/blog/embedding-quantization)
- [Qdrant Binary Quantization](https://qdrant.tech/articles/binary-quantization/)
- [Weaviate 32x Memory Reduction](https://weaviate.io/blog/binary-quantization)

### Hybrid Search
- [Elastic Hybrid Search Guide](https://www.elastic.co/what-is/hybrid-search)
- [Weaviate Hybrid Search Explained](https://weaviate.io/blog/hybrid-search-explained)
- [ParadeDB RRF Explanation](https://www.paradedb.com/learn/search-concepts/reciprocal-rank-fusion)

### Embedding Models
- [all-MiniLM-L6-v2 on HuggingFace](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)
- [Open Source Embedding Models Benchmark](https://research.aimultiple.com/open-source-embedding-models/)
- [DataCamp Vector Databases 2026](https://www.datacamp.com/blog/the-top-5-vector-databases)

### Incremental Indexing
- [CockroachDB C-SPANN Real-time Indexing](https://www.cockroachlabs.com/blog/cspann-real-time-indexing-billions-vectors/)
- [CocoIndex Architecture](https://medium.com/@cocoindex.io/building-a-real-time-data-substrate-for-ai-agents-the-architecture-behind-cocoindex-729981f0f3a4)
