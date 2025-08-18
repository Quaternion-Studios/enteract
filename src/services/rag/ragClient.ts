import { invoke } from '@tauri-apps/api/core'
import type { 
  EnhancedDocument, 
  EnhancedDocumentChunk, 
  RagSettings, 
  StorageStats, 
  EmbeddingStatus, 
  SearchResponse,
  DocumentValidationResult 
} from './types'

export class RagClient {
  private static instance: RagClient
  private isInitialized = false

  private constructor() {}

  static getInstance(): RagClient {
    if (!RagClient.instance) {
      RagClient.instance = new RagClient()
    }
    return RagClient.instance
  }

  async initialize(): Promise<void> {
    if (this.isInitialized) return

    try {
      await invoke('initialize_enhanced_rag_system')
      this.isInitialized = true
    } catch (error) {
      console.error('Failed to initialize RAG system:', error)
      throw error
    }
  }

  async uploadDocument(
    fileName: string,
    content: string,
    fileType: string,
    fileSize: number,
    metadata?: Record<string, any>
  ): Promise<EnhancedDocument> {
    await this.ensureInitialized()
    
    return invoke('upload_enhanced_document', {
      fileName,
      content,
      fileType,
      fileSize,
      metadata: metadata ? JSON.stringify(metadata) : undefined
    })
  }

  async getAllDocuments(): Promise<EnhancedDocument[]> {
    await this.ensureInitialized()
    return invoke('get_all_enhanced_documents')
  }

  async deleteDocument(documentId: string): Promise<void> {
    await this.ensureInitialized()
    return invoke('delete_enhanced_document', { documentId })
  }

  async searchDocuments(
    query: string,
    documentIds?: string[],
    maxResults?: number
  ): Promise<EnhancedDocumentChunk[]> {
    await this.ensureInitialized()
    
    return invoke('search_enhanced_documents', {
      query,
      documentIds,
      maxResults
    })
  }

  async generateEmbeddings(documentIds?: string[]): Promise<void> {
    await this.ensureInitialized()
    return invoke('generate_enhanced_embeddings', { documentIds })
  }

  async getSettings(): Promise<RagSettings> {
    await this.ensureInitialized()
    return invoke('get_enhanced_rag_settings')
  }

  async updateSettings(settings: Partial<RagSettings>): Promise<void> {
    await this.ensureInitialized()
    return invoke('update_enhanced_rag_settings', { settings })
  }

  async getStorageStats(): Promise<StorageStats> {
    await this.ensureInitialized()
    return invoke('get_enhanced_storage_stats')
  }

  async getEmbeddingStatus(): Promise<EmbeddingStatus> {
    await this.ensureInitialized()
    return invoke('get_embedding_status')
  }

  async validateDocuments(documentIds: string[]): Promise<DocumentValidationResult> {
    await this.ensureInitialized()
    return invoke('ensure_documents_ready_for_search', { documentIds })
  }

  async checkDocumentDuplicate(content: string): Promise<string | null> {
    await this.ensureInitialized()
    return invoke('check_document_duplicate', { content })
  }

  async clearEmbeddingCache(): Promise<void> {
    await this.ensureInitialized()
    return invoke('clear_enhanced_embedding_cache')
  }

  private async ensureInitialized(): Promise<void> {
    if (!this.isInitialized) {
      await this.initialize()
    }
  }
}

// Export singleton instance
export const ragClient = RagClient.getInstance()