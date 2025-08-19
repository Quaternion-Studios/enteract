import { invoke } from '@tauri-apps/api/core'

export interface Document {
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
  metadata: string | null
}

export interface DocumentChunk {
  id: string
  document_id: string
  chunk_index: number
  content: string
  start_char: number
  end_char: number
  embedding: number[] | null
  metadata: string | null
}

export interface RagSettings {
  max_document_size_mb: number
  max_collection_size_gb: number
  max_cached_documents: number
  chunk_size: number
  chunk_overlap: number
  auto_embedding: boolean
  background_processing: boolean
}

export interface StorageStats {
  total_documents: number
  total_size_bytes: number
  total_size_mb: number
  cached_documents: number
  max_cached_documents: number
  max_document_size_mb: number
  max_collection_size_gb: number
}

class RagService {
  async initialize(): Promise<void> {
    try {
      await invoke('initialize_rag_system')
      console.log('RAG system initialized')
    } catch (error) {
      console.error('Failed to initialize RAG system:', error)
      throw error
    }
  }

  async uploadDocument(file: File): Promise<Document> {
    try {
      // Convert file to base64 for transfer to Rust backend
      const arrayBuffer = await file.arrayBuffer()
      const uint8Array = new Uint8Array(arrayBuffer)
      
      const document = await invoke<Document>('upload_document', {
        fileName: file.name,
        fileContent: Array.from(uint8Array),
        fileType: file.type
      })
      
      console.log('Document uploaded:', document)
      return document
    } catch (error) {
      console.error('Failed to upload document:', error)
      throw error
    }
  }

  async getAllDocuments(): Promise<Document[]> {
    try {
      const documents = await invoke<Document[]>('get_all_documents')
      return documents
    } catch (error) {
      console.error('Failed to get documents:', error)
      throw error
    }
  }

  async deleteDocument(documentId: string): Promise<void> {
    try {
      await invoke('delete_document', { documentId })
      console.log(`Document ${documentId} deleted`)
    } catch (error) {
      console.error('Failed to delete document:', error)
      throw error
    }
  }

  async searchDocuments(
    query: string,
    contextDocumentIds: string[] = []
  ): Promise<DocumentChunk[]> {
    try {
      const chunks = await invoke<DocumentChunk[]>('search_documents', {
        query,
        contextDocumentIds
      })
      return chunks
    } catch (error) {
      console.error('Failed to search documents:', error)
      throw error
    }
  }

  async updateSettings(settings: RagSettings): Promise<void> {
    try {
      await invoke('update_rag_settings', { settings })
      console.log('RAG settings updated')
    } catch (error) {
      console.error('Failed to update settings:', error)
      throw error
    }
  }

  async getSettings(): Promise<RagSettings> {
    try {
      const settings = await invoke<RagSettings>('get_rag_settings')
      return settings
    } catch (error) {
      console.error('Failed to get settings:', error)
      throw error
    }
  }

  async getStorageStats(): Promise<StorageStats> {
    try {
      const stats = await invoke<StorageStats>('get_storage_stats')
      return stats
    } catch (error) {
      console.error('Failed to get storage stats:', error)
      throw error
    }
  }

  async generateEmbeddings(documentId: string): Promise<void> {
    try {
      await invoke('generate_embeddings', { documentId })
      console.log(`Embeddings generated for document ${documentId}`)
    } catch (error) {
      console.error('Failed to generate embeddings:', error)
      throw error
    }
  }

  async clearEmbeddingCache(): Promise<void> {
    try {
      await invoke('clear_embedding_cache')
      console.log('Embedding cache cleared')
    } catch (error) {
      console.error('Failed to clear embedding cache:', error)
      throw error
    }
  }

  // Helper method to format file size
  formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  }

  // Helper method to extract text from various file types
  async extractTextContent(file: File): Promise<string> {
    if (file.type.includes('text') || file.type.includes('plain')) {
      return await file.text()
    }
    
    // For other file types, the backend will handle extraction
    return ''
  }

  // Helper method to validate file before upload
  validateFile(file: File, settings?: RagSettings): { valid: boolean; error?: string } {
    if (!settings) {
      return { valid: true }
    }

    const fileSizeMB = file.size / (1024 * 1024)
    if (fileSizeMB > settings.max_document_size_mb) {
      return {
        valid: false,
        error: `File size ${fileSizeMB.toFixed(2)}MB exceeds limit of ${settings.max_document_size_mb}MB`
      }
    }

    // Add more validation as needed
    const supportedTypes = [
      'text/', 'application/pdf', 'image/', 
      'application/msword', 'application/vnd.openxmlformats-officedocument'
    ]
    
    const isSupported = supportedTypes.some(type => file.type.includes(type))
    if (!isSupported) {
      return {
        valid: false,
        error: `File type ${file.type} is not supported`
      }
    }

    return { valid: true }
  }
}

export const ragService = new RagService()
export default ragService