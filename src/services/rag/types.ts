// Frontend types for RAG system
export interface EnhancedDocument {
  id: string
  file_name: string
  file_path: string
  file_type: string
  file_size: number
  content: string
  created_at: string
  updated_at: string
  access_count: number
  last_accessed?: string
  is_cached: boolean
  embedding_status: 'pending' | 'processing' | 'completed' | 'failed'
  chunk_count: number
  metadata?: string
  content_hash?: string
}

export interface EnhancedDocumentChunk {
  id: string
  document_id: string
  chunk_index: number
  content: string
  start_char: number
  end_char: number
  token_count: number
  embedding?: number[]
  similarity_score?: number
  bm25_score?: number
  metadata?: string
}

export interface RagSettings {
  max_document_size_mb: number
  max_collection_size_gb: number
  max_cached_documents: number
  auto_embedding: boolean
  background_processing: boolean
  reranking_enabled: boolean
  chunking_config: ChunkingConfig
  embedding_config: EmbeddingConfig
  search_config: SearchConfig
}

export interface ChunkingConfig {
  chunk_size: number
  chunk_overlap: number
  min_chunk_size: number
  max_chunk_size: number
  sentence_splitter: boolean
  paragraph_splitter: boolean
}

export interface EmbeddingConfig {
  model_name: string
  batch_size: number
  max_tokens: number
  normalize: boolean
}

export interface SearchConfig {
  max_results: number
  similarity_threshold: number
  bm25_weight: number
  vector_weight: number
  rerank_top_k: number
}

export interface StorageStats {
  total_documents: number
  total_chunks: number
  database_size_mb: number
  storage_size_mb: number
  index_size_mb: number
  cache_size_mb: number
  last_updated: string
}

export interface EmbeddingStatus {
  total_documents: number
  processed_documents: number
  pending_documents: number
  failed_documents: number
  processing_documents: number
  percentage_complete: number
}

export interface SearchResponse {
  chunks: EnhancedDocumentChunk[]
  total_results: number
  search_time_ms: number
  query: string
}

export interface DocumentValidationResult {
  ready_documents: string[]
  pending_documents: string[]
  processing_documents: string[]
  failed_documents: string[]
}