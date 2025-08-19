import { invoke } from '@tauri-apps/api/core'

// Enhanced Document interfaces with additional fields
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
  last_accessed: string | null
  is_cached: boolean
  embedding_status: 'pending' | 'processing' | 'completed' | 'failed'
  chunk_count: number
  metadata: string | null
}

export interface EnhancedDocumentChunk {
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

// Configuration interfaces
export interface ChunkingConfig {
  chunk_size: number
  chunk_overlap: number
  max_chunk_size: number
  min_chunk_size: number
  respect_sentence_boundaries: boolean
  respect_paragraph_boundaries: boolean
}

export interface EmbeddingConfig {
  model_name: string
  max_length: number
  normalize_embeddings: boolean
  show_download_progress: boolean
}

export interface SearchConfig {
  bm25_weight: number
  vector_weight: number
  max_results: number
  min_score_threshold: number
}

export interface EnhancedRagSettings {
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

export interface EnhancedStorageStats {
  total_documents: number
  total_size_bytes: number
  total_size_mb: number
  cached_documents: number
  total_chunks: number
  embedded_chunks: number
  embedding_coverage: number
  max_cached_documents: number
  max_document_size_mb: number
  embedding_model: string
  reranking_enabled: boolean
}

export interface EmbeddingStatus {
  total_documents: number
  completed_documents: number
  processing_documents: number
  failed_documents: number
  completion_percentage: number
}

export interface FileValidation {
  valid: boolean
  size_valid: boolean
  type_valid: boolean
  file_size_mb: number
  max_size_mb: number
  supported_types: string[]
  error?: string
}

class EnhancedRagService {
  private initialized = false

  async initialize(): Promise<void> {
    try {
      await invoke('initialize_rag_system')
      this.initialized = true
      console.log('Enhanced RAG system initialized')
    } catch (error) {
      console.error('Failed to initialize Enhanced RAG system:', error)
      throw error
    }
  }

  async uploadDocument(file: File): Promise<EnhancedDocument> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      // Convert file to base64 for transfer to Rust backend
      const arrayBuffer = await file.arrayBuffer()
      const uint8Array = new Uint8Array(arrayBuffer)
      
      const document = await invoke<EnhancedDocument>('upload_document', {
        fileName: file.name,
        fileContent: Array.from(uint8Array),
        fileType: file.type
      })
      
      console.log('Enhanced document uploaded:', document)
      return document
    } catch (error) {
      console.error('Failed to upload enhanced document:', error)
      throw error
    }
  }

  async getAllDocuments(): Promise<EnhancedDocument[]> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const documents = await invoke<EnhancedDocument[]>('get_all_documents')
      return documents
    } catch (error) {
      console.error('Failed to get enhanced documents:', error)
      throw error
    }
  }

  async deleteDocument(documentId: string): Promise<void> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      await invoke('delete_document', { documentId: documentId })
      console.log(`Enhanced document ${documentId} deleted`)
    } catch (error) {
      console.error('Failed to delete enhanced document:', error)
      throw error
    }
  }

  async searchDocuments(
    query: string,
    contextDocumentIds: string[] = []
  ): Promise<EnhancedDocumentChunk[]> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const chunks = await invoke<EnhancedDocumentChunk[]>('search_documents', {
        query,
        contextDocumentIds: contextDocumentIds
      })
      return chunks
    } catch (error) {
      console.error('Failed to search enhanced documents:', error)
      throw error
    }
  }

  async updateSettings(settings: EnhancedRagSettings): Promise<void> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      await invoke('update_rag_settings', { settings })
      console.log('Enhanced RAG settings updated')
    } catch (error) {
      console.error('Failed to update enhanced settings:', error)
      throw error
    }
  }

  async getSettings(): Promise<EnhancedRagSettings> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const settings = await invoke<EnhancedRagSettings>('get_rag_settings')
      return settings
    } catch (error) {
      console.error('Failed to get enhanced settings:', error)
      throw error
    }
  }

  async getStorageStats(): Promise<EnhancedStorageStats> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const stats = await invoke<EnhancedStorageStats>('get_storage_stats')
      return stats
    } catch (error) {
      console.error('Failed to get enhanced storage stats:', error)
      throw error
    }
  }

  async generateEmbeddings(documentId: string): Promise<void> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      await invoke('generate_embeddings', { documentId: documentId })
      console.log(`Enhanced embeddings generated for document ${documentId}`)
    } catch (error) {
      console.error('Failed to generate enhanced embeddings:', error)
      throw error
    }
  }

  async clearEmbeddingCache(): Promise<void> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      await invoke('clear_embedding_cache')
      console.log('Enhanced embedding cache cleared')
    } catch (error) {
      console.error('Failed to clear enhanced embedding cache:', error)
      throw error
    }
  }

  async getEmbeddingStatus(): Promise<EmbeddingStatus> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const status = await invoke<EmbeddingStatus>('get_embedding_status')
      return status
    } catch (error) {
      console.error('Failed to get embedding status:', error)
      throw error
    }
  }

  async validateFileUpload(file: File): Promise<FileValidation> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const validation = await invoke<FileValidation>('validate_rag_file_upload', {
        fileName: file.name,
        fileSize: file.size,
        fileType: file.type
      })
      return validation
    } catch (error) {
      console.error('Failed to validate file upload:', error)
      throw error
    }
  }

  async checkDocumentDuplicate(file: File): Promise<{ isDuplicate: boolean; existingDocument?: EnhancedDocument }> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }
      
      const fileContent = await this.fileToArrayBuffer(file)
      const uint8Array = new Uint8Array(fileContent)
      
      const result = await invoke<Record<string, any>>('check_document_duplicate', {
        fileName: file.name,
        fileContent: Array.from(uint8Array)
      })
      
      return {
        isDuplicate: result.is_duplicate as boolean,
        existingDocument: result.existing_document as EnhancedDocument | undefined
      }
    } catch (error) {
      console.error('Duplicate check failed:', error)
      return { isDuplicate: false }
    }
  }

  async getDocumentEmbeddingStatus(documentIds: string[]): Promise<Record<string, string>> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const statusMap = await invoke<Record<string, string>>('get_document_embedding_status', {
        documentIds: documentIds
      })
      return statusMap
    } catch (error) {
      console.error('Failed to get document embedding status:', error)
      throw error
    }
  }

  async ensureDocumentsReadyForSearch(documentIds: string[]): Promise<Record<string, string>> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      const readinessMap = await invoke<Record<string, string>>('ensure_documents_ready_for_search', {
        documentIds: documentIds
      })
      return readinessMap
    } catch (error) {
      console.error('Failed to ensure documents ready for search:', error)
      throw error
    }
  }

  async generateEmbeddingsForSelection(documentIds: string[]): Promise<void> {
    try {
      if (!this.initialized) {
        await this.initialize()
      }

      await invoke('generate_embeddings_for_selection', { documentIds: documentIds })
      console.log(`Priority embeddings triggered for ${documentIds.length} documents`)
    } catch (error) {
      console.error('Failed to generate embeddings for selection:', error)
      throw error
    }
  }

  // Helper methods
  private async fileToArrayBuffer(file: File): Promise<ArrayBuffer> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(reader.result as ArrayBuffer)
      reader.onerror = reject
      reader.readAsArrayBuffer(file)
    })
  }

  formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  }

  getEmbeddingStatusColor(status: string): string {
    switch (status) {
      case 'completed': return 'text-green-400'
      case 'processing': return 'text-yellow-400'
      case 'failed': return 'text-red-400'
      case 'pending': 
      default: return 'text-gray-400'
    }
  }

  getEmbeddingStatusIcon(status: string): string {
    switch (status) {
      case 'completed': return '✅'
      case 'processing': return '⚡'
      case 'failed': return '❌'
      case 'pending': 
      default: return '⏳'
    }
  }

  formatContextForAI(chunks: EnhancedDocumentChunk[]): string {
    if (chunks.length === 0) return ''
    
    // Group chunks by document
    const documentChunks = chunks.reduce((acc, chunk) => {
      if (!acc[chunk.document_id]) {
        acc[chunk.document_id] = []
      }
      acc[chunk.document_id].push(chunk)
      return acc
    }, {} as Record<string, EnhancedDocumentChunk[]>)
    
    let context = 'Relevant document context:\n\n'
    
    for (const [docId, docChunks] of Object.entries(documentChunks)) {
      // Sort chunks by similarity score for better context ordering
      docChunks.sort((a, b) => (b.similarity_score || 0) - (a.similarity_score || 0))
      
      context += `From document ${docId}:\n`
      docChunks.forEach((chunk, index) => {
        const score = chunk.similarity_score ? ` (similarity: ${chunk.similarity_score.toFixed(3)})` : ''
        context += `${index + 1}. ${chunk.content.trim()}${score}\n`
      })
      context += '\n'
    }
    
    return context
  }

  // Get default settings
  getDefaultSettings(): EnhancedRagSettings {
    return {
      max_document_size_mb: 50.0,
      max_collection_size_gb: 2.0,
      max_cached_documents: 10,
      auto_embedding: true,
      background_processing: true,
      reranking_enabled: false,
      chunking_config: {
        chunk_size: 512,
        chunk_overlap: 64,
        max_chunk_size: 800,
        min_chunk_size: 100,
        respect_sentence_boundaries: true,
        respect_paragraph_boundaries: true
      },
      embedding_config: {
        model_name: "BAAI/bge-small-en-v1.5",
        max_length: 512,
        normalize_embeddings: true,
        show_download_progress: true
      },
      search_config: {
        bm25_weight: 0.7,
        vector_weight: 0.3,
        max_results: 50,
        min_score_threshold: 0.1
      }
    }
  }

  // Migration utility to upgrade from legacy RAG system
  async migrateFromLegacy(): Promise<void> {
    try {
      console.log('Migration from legacy RAG system not yet implemented')
      // TODO: Implement migration logic if needed
    } catch (error) {
      console.error('Failed to migrate from legacy RAG system:', error)
      throw error
    }
  }

  // Performance monitoring
  async getPerformanceMetrics(): Promise<any> {
    try {
      // This could be extended to include performance metrics
      const stats = await this.getStorageStats()
      const embeddingStatus = await this.getEmbeddingStatus()
      
      return {
        ...stats,
        ...embeddingStatus,
        performance: {
          search_latency_ms: 0, // TODO: Implement actual metrics
          embedding_throughput: 0,
          cache_hit_rate: stats.embedding_coverage
        }
      }
    } catch (error) {
      console.error('Failed to get performance metrics:', error)
      throw error
    }
  }
}

export const enhancedRagService = new EnhancedRagService()
export default enhancedRagService