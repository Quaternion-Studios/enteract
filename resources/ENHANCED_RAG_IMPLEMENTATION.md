# Enhanced RAG System Implementation Documentation

## Overview

This document details the implementation of a comprehensive, high-performance RAG (Retrieval-Augmented Generation) system for the Enteract desktop application. The system has been designed following best practices for local-first, fast, and lightweight operation on Windows desktop environments.

## Architecture Summary

The enhanced RAG system implements a **hybrid search architecture** combining:
- **FastEmbed** with BGE-small-en-v1.5 (384-dimensional embeddings) for semantic search
- **Tantivy** for BM25 full-text search and vector storage
- **Reciprocal Rank Fusion (RRF)** for combining BM25 and vector search results
- **Intelligent chunking** with sentence/paragraph boundary awareness
- **Optional reranking** capabilities for enhanced result quality

## Key Features

### ✅ Completed Features

1. **FastEmbed Integration**
   - BGE-small-en-v1.5 model (384 dimensions)
   - CPU-optimized ONNX Runtime execution
   - Local model caching
   - Automatic model downloading

2. **Tantivy Hybrid Search**
   - BM25 full-text search
   - Vector similarity search with cosine similarity
   - Hybrid result fusion using RRF
   - Configurable search weights

3. **Enhanced Document Processing**
   - Intelligent chunking with sentence/paragraph boundaries
   - Token-aware chunking using tiktoken
   - PDF text extraction
   - Multiple file format support

4. **Advanced Database Schema**
   - Enhanced document metadata tracking
   - Embedding status tracking
   - Background processing queue
   - Performance optimized indexes

5. **Frontend Integration**
   - Dual service architecture (legacy + enhanced)
   - Enhanced TypeScript interfaces
   - Real-time embedding status tracking
   - File validation with backend integration

### 🔄 Pending Features

1. **Reranking Integration** (Optional)
   - BGE-reranker-base model support
   - Configurable reranking toggle
   - Top-k candidate reranking

## Technical Implementation

### Backend Architecture

#### Core Services

1. **EmbeddingService** (`embedding_service.rs`)
   - Manages FastEmbed model lifecycle
   - Handles document and query embedding generation
   - Supports multiple embedding models
   - CPU-optimized with configurable batch processing

2. **SearchService** (`search_service.rs`)
   - Tantivy index management
   - Hybrid BM25 + vector search
   - Reciprocal Rank Fusion implementation
   - Configurable search parameters

3. **ChunkingService** (`chunking_service.rs`)
   - Intelligent text chunking
   - Sentence and paragraph boundary respect
   - Token counting with tiktoken
   - Configurable chunk sizes and overlap

4. **EnhancedRagSystem** (`enhanced_rag_system.rs`)
   - Orchestrates all RAG components
   - Manages document lifecycle
   - Background embedding processing
   - Performance monitoring

#### Database Schema

```sql
-- Enhanced Documents Table
CREATE TABLE enhanced_documents (
    id TEXT PRIMARY KEY,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    access_count INTEGER DEFAULT 0,
    last_accessed TEXT,
    is_cached INTEGER DEFAULT 0,
    embedding_status TEXT DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed'
    chunk_count INTEGER DEFAULT 0,
    metadata TEXT
);

-- Enhanced Document Chunks Table
CREATE TABLE enhanced_document_chunks (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    start_char INTEGER NOT NULL,
    end_char INTEGER NOT NULL,
    token_count INTEGER NOT NULL,
    embedding BLOB, -- Serialized f32 vector
    metadata TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (document_id) REFERENCES enhanced_documents(id) ON DELETE CASCADE
);

-- Background Processing Queue
CREATE TABLE processing_queue (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    task_type TEXT NOT NULL, -- 'embedding_generation', 'reranking'
    status TEXT DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed'
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    FOREIGN KEY (document_id) REFERENCES enhanced_documents(id) ON DELETE CASCADE
);
```

### Frontend Architecture

#### Service Layer

1. **EnhancedRagService** (`enhancedRagService.ts`)
   - TypeScript interface to enhanced backend
   - Automatic service initialization
   - File validation and upload progress
   - Enhanced error handling

2. **useRagDocuments** Composable (Updated)
   - Dual service support (legacy + enhanced)
   - Enhanced type safety
   - Real-time status tracking
   - Backward compatibility

#### Key TypeScript Interfaces

```typescript
interface EnhancedDocument {
  id: string
  file_name: string
  file_path: string
  file_type: string
  file_size: number
  content: string
  created_at: string
  updated_at: string
  access_count: number
  last_accessed: string | null
  is_cached: boolean
  embedding_status: 'pending' | 'processing' | 'completed' | 'failed'
  chunk_count: number
  metadata: string | null
}

interface EnhancedDocumentChunk {
  id: string
  document_id: string
  chunk_index: number
  content: string
  start_char: number
  end_char: number
  token_count: number
  embedding: number[] | null
  similarity_score: number | null
  bm25_score: number | null
  metadata: string | null
}
```

### Search Algorithm

#### Hybrid Search Process

1. **Query Processing**
   ```rust
   // Generate query embedding
   let query_embedding = embedding_service.embed_query(query)?;
   
   // Perform BM25 search
   let bm25_results = search_service.search_bm25(query, limit * 2)?;
   
   // Perform vector search
   let vector_results = search_service.search_vector(&query_embedding, limit * 2)?;
   
   // Apply Reciprocal Rank Fusion
   let fused_results = reciprocal_rank_fusion(bm25_results, vector_results, limit)?;
   ```

2. **Reciprocal Rank Fusion**
   ```rust
   fn reciprocal_rank_fusion(
       bm25_results: Vec<SearchResult>,
       vector_results: Vec<SearchResult>,
       limit: usize,
   ) -> Vec<SearchResult> {
       let k = 60.0; // RRF parameter
       let mut score_map: HashMap<String, f32> = HashMap::new();
       
       // Process BM25 results
       for (rank, result) in bm25_results.into_iter().enumerate() {
           let rrf_score = bm25_weight / (k + rank as f32 + 1.0);
           score_map.insert(result.chunk_id.clone(), rrf_score);
       }
       
       // Process vector results and merge
       for (rank, result) in vector_results.into_iter().enumerate() {
           let rrf_score = vector_weight / (k + rank as f32 + 1.0);
           let existing_score = score_map.get(&result.chunk_id).unwrap_or(&0.0);
           score_map.insert(result.chunk_id.clone(), existing_score + rrf_score);
       }
       
       // Sort by final score and return top results
       // ... sorting and truncation logic
   }
   ```

### Configuration

#### Default Settings

```rust
impl Default for EnhancedRagSettings {
    fn default() -> Self {
        Self {
            max_document_size_mb: 50.0,
            max_collection_size_gb: 2.0,
            max_cached_documents: 10,
            auto_embedding: true,
            background_processing: true,
            reranking_enabled: false,
            chunking_config: ChunkingConfig {
                chunk_size: 512,
                chunk_overlap: 64,
                max_chunk_size: 800,
                min_chunk_size: 100,
                respect_sentence_boundaries: true,
                respect_paragraph_boundaries: true,
            },
            embedding_config: EmbeddingConfig {
                model_name: "BAAI/bge-small-en-v1.5".to_string(),
                max_length: 512,
                normalize_embeddings: true,
                show_download_progress: true,
            },
            search_config: SearchConfig {
                bm25_weight: 0.7,
                vector_weight: 0.3,
                max_results: 50,
                min_score_threshold: 0.1,
            },
        }
    }
}
```

## Performance Characteristics

### Model Specifications

- **Embedding Model**: BGE-small-en-v1.5
- **Model Size**: ~133MB
- **Embedding Dimension**: 384
- **Max Sequence Length**: 512 tokens
- **Language**: English (optimized)

### Expected Performance

- **Embedding Generation**: ~100-500 docs/second (CPU-dependent)
- **Search Latency**: <100ms for typical queries
- **Memory Usage**: ~200MB base + 1MB per 1000 documents
- **Storage**: ~1KB per chunk + 1.5KB per embedding

### Optimization Features

1. **Lazy Loading**: Models loaded on first use
2. **Background Processing**: Embeddings generated asynchronously
3. **Efficient Serialization**: Binary embedding storage
4. **Index Optimization**: Tantivy MMAP for fast search
5. **Chunking Optimization**: Sentence boundary respect reduces fragmentation

## Usage Guide

### Basic Usage

1. **Initialize the System**
   ```typescript
   const ragDocuments = useRagDocuments()
   await ragDocuments.initialize()
   ```

2. **Upload Documents**
   ```typescript
   const file = // File object
   const document = await ragDocuments.uploadDocument(file)
   ```

3. **Search Documents**
   ```typescript
   const results = await ragDocuments.searchDocuments("your query", true)
   ```

4. **Use in AI Context**
   ```typescript
   const contextText = ragDocuments.formatContextForAI(results)
   // Include contextText in your AI prompt
   ```

### Advanced Configuration

```typescript
// Update settings
await ragDocuments.updateSettings({
  search_config: {
    bm25_weight: 0.8,
    vector_weight: 0.2,
    max_results: 20,
    min_score_threshold: 0.15
  },
  chunking_config: {
    chunk_size: 256,
    chunk_overlap: 32,
    respect_sentence_boundaries: true
  }
})
```

### Monitoring

```typescript
// Get embedding status
const status = await ragDocuments.getEmbeddingStatus()
console.log(`${status.completion_percentage}% complete`)

// Get storage statistics
const stats = await ragDocuments.getStorageStats()
console.log(`${stats.total_documents} documents, ${stats.embedded_chunks} embedded`)
```

## Migration from Legacy System

The enhanced system maintains backward compatibility with the existing RAG implementation. The `useRagDocuments` composable automatically uses the enhanced system when available, falling back to the legacy system if needed.

### Migration Path

1. **Automatic Migration**: The system will automatically use enhanced features when available
2. **Data Compatibility**: Existing documents remain accessible
3. **Progressive Enhancement**: New uploads use enhanced processing
4. **Settings Migration**: Legacy settings are automatically converted

## Troubleshooting

### Common Issues

1. **Model Download Failures**
   - Ensure internet connectivity
   - Check available disk space (>500MB)
   - Verify antivirus is not blocking downloads

2. **Slow Embedding Generation**
   - Check CPU usage and available cores
   - Consider reducing batch size in settings
   - Ensure sufficient RAM (>4GB recommended)

3. **Search Quality Issues**
   - Adjust BM25/vector weights in search config
   - Increase max_results for broader search
   - Lower min_score_threshold for more results

### Performance Optimization

1. **For Better Speed**
   ```typescript
   await ragDocuments.updateSettings({
     chunking_config: { chunk_size: 256 }, // Smaller chunks
     search_config: { max_results: 20 },   // Fewer results
     background_processing: true           // Async processing
   })
   ```

2. **For Better Quality**
   ```typescript
   await ragDocuments.updateSettings({
     chunking_config: { 
       chunk_size: 512,
       respect_sentence_boundaries: true 
     },
     search_config: { 
       bm25_weight: 0.6,
       vector_weight: 0.4,
       max_results: 50 
     },
     reranking_enabled: true // When implemented
   })
   ```

## Future Enhancements

### Planned Features

1. **Reranking Integration**
   - BGE-reranker-base model
   - Configurable reranking for top-k results
   - Quality vs. performance tradeoffs

2. **Multi-Query Expansion**
   - Automatic query reformulation
   - Improved recall for complex queries

3. **Semantic Caching**
   - Cache similar query results
   - Reduce computation for repeated searches

4. **Advanced Analytics**
   - Query performance metrics
   - Document access patterns
   - Search quality scoring

### Potential Optimizations

1. **GPU Acceleration** (Optional)
   - CUDA support for faster embeddings
   - Configurable CPU/GPU selection

2. **Quantized Models**
   - 8-bit quantization for reduced memory
   - Faster inference with minimal quality loss

3. **Approximate Search**
   - HNSW index for vector search
   - Configurable accuracy/speed tradeoffs

## Conclusion

The enhanced RAG system provides a significant upgrade over the legacy implementation, offering:

- **Better Search Quality**: Hybrid BM25 + vector search with RRF
- **Improved Performance**: Optimized embeddings and indexing
- **Enhanced UX**: Real-time status tracking and validation
- **Future-Proof Architecture**: Extensible design for advanced features

The system is designed to grow with your application's needs while maintaining excellent performance on desktop hardware.

---