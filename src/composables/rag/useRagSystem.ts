import { ref, reactive, computed } from 'vue'
import { ragClient } from '@/services/rag/ragClient'
import { RagContextManager, type RagContextResult } from '@/services/rag/contextManager'
import type { 
  EnhancedDocument, 
  EnhancedDocumentChunk, 
  RagSettings, 
  StorageStats, 
  EmbeddingStatus 
} from '@/services/rag/types'

interface RagState {
  documents: EnhancedDocument[]
  isInitialized: boolean
  isLoading: boolean
  settings: RagSettings | null
  stats: StorageStats | null
  embeddingStatus: EmbeddingStatus | null
  error: string | null
}

const state = reactive<RagState>({
  documents: [],
  isInitialized: false,
  isLoading: false,
  settings: null,
  stats: null,
  embeddingStatus: null,
  error: null
})

export function useRagSystem() {
  const selectedDocumentIds = ref<string[]>([])

  // Computed properties
  const hasDocuments = computed(() => state.documents.length > 0)
  const hasSelectedDocuments = computed(() => selectedDocumentIds.value.length > 0)
  const readyDocuments = computed(() => 
    state.documents.filter(doc => doc.embedding_status === 'completed')
  )
  const isProcessing = computed(() => 
    state.documents.some(doc => doc.embedding_status === 'processing')
  )

  // Initialize the RAG system
  const initialize = async (): Promise<void> => {
    if (state.isInitialized) return

    try {
      state.isLoading = true
      state.error = null
      
      await ragClient.initialize()
      await loadDocuments()
      await loadSettings()
      
      state.isInitialized = true
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to initialize RAG system'
      console.error('RAG initialization error:', error)
      throw error
    } finally {
      state.isLoading = false
    }
  }

  // Load all documents
  const loadDocuments = async (): Promise<void> => {
    try {
      state.documents = await ragClient.getAllDocuments()
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to load documents'
      throw error
    }
  }

  // Upload a new document
  const uploadDocument = async (
    file: File,
    metadata?: Record<string, any>
  ): Promise<EnhancedDocument> => {
    try {
      state.isLoading = true
      state.error = null

      const content = await file.text()
      
      // Check for duplicates
      const existingId = await ragClient.checkDocumentDuplicate(content)
      if (existingId) {
        throw new Error(`Document already exists with ID: ${existingId}`)
      }

      const document = await ragClient.uploadDocument(
        file.name,
        content,
        file.type || 'text/plain',
        file.size,
        metadata
      )

      // Add to local state
      state.documents.unshift(document)

      // Auto-generate embeddings if enabled
      if (state.settings?.auto_embedding) {
        await generateEmbeddings([document.id])
      }

      return document
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to upload document'
      throw error
    } finally {
      state.isLoading = false
    }
  }

  // Delete a document
  const deleteDocument = async (documentId: string): Promise<void> => {
    try {
      state.isLoading = true
      state.error = null

      await ragClient.deleteDocument(documentId)
      
      // Remove from local state
      state.documents = state.documents.filter(doc => doc.id !== documentId)
      selectedDocumentIds.value = selectedDocumentIds.value.filter(id => id !== documentId)
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to delete document'
      throw error
    } finally {
      state.isLoading = false
    }
  }

  // Search documents with smart context management
  const searchDocuments = async (
    query: string,
    maxRagTokens?: number
  ): Promise<RagContextResult> => {
    try {
      const documentsToSearch = hasSelectedDocuments.value 
        ? selectedDocumentIds.value 
        : readyDocuments.value.map(doc => doc.id)

      if (documentsToSearch.length === 0) {
        return {
          context: '',
          tokensUsed: 0,
          chunksIncluded: 0,
          chunksDropped: 0
        }
      }

      const chunks = await ragClient.searchDocuments(query, documentsToSearch)
      
      // Use smart context management
      return RagContextManager.formatContextForAI(chunks, maxRagTokens)
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Search failed'
      throw error
    }
  }

  // Generate embeddings for documents
  const generateEmbeddings = async (documentIds?: string[]): Promise<void> => {
    try {
      state.isLoading = true
      await ragClient.generateEmbeddings(documentIds)
      
      // Update document statuses
      await loadDocuments()
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to generate embeddings'
      throw error
    } finally {
      state.isLoading = false
    }
  }

  // Load settings
  const loadSettings = async (): Promise<void> => {
    try {
      state.settings = await ragClient.getSettings()
    } catch (error) {
      console.error('Failed to load RAG settings:', error)
    }
  }

  // Update settings
  const updateSettings = async (newSettings: Partial<RagSettings>): Promise<void> => {
    try {
      await ragClient.updateSettings(newSettings)
      await loadSettings()
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Failed to update settings'
      throw error
    }
  }

  // Load statistics
  const loadStats = async (): Promise<void> => {
    try {
      state.stats = await ragClient.getStorageStats()
    } catch (error) {
      console.error('Failed to load storage stats:', error)
    }
  }

  // Load embedding status
  const loadEmbeddingStatus = async (): Promise<void> => {
    try {
      state.embeddingStatus = await ragClient.getEmbeddingStatus()
    } catch (error) {
      console.error('Failed to load embedding status:', error)
    }
  }

  // Select/deselect documents
  const toggleDocumentSelection = (documentId: string): void => {
    const index = selectedDocumentIds.value.indexOf(documentId)
    if (index > -1) {
      selectedDocumentIds.value.splice(index, 1)
    } else {
      // Limit to 5 documents max
      if (selectedDocumentIds.value.length >= 5) {
        state.error = 'Maximum 5 documents can be selected for search'
        return
      }
      selectedDocumentIds.value.push(documentId)
    }
  }

  const selectAllDocuments = (): void => {
    const readyIds = readyDocuments.value.map(doc => doc.id).slice(0, 5)
    selectedDocumentIds.value = readyIds
  }

  const clearSelection = (): void => {
    selectedDocumentIds.value = []
  }

  // Clear errors
  const clearError = (): void => {
    state.error = null
  }

  // Calculate optimal token allocation
  const calculateTokenAllocation = (
    ragChunks: EnhancedDocumentChunk[],
    totalTokens: number = 4000
  ) => {
    return RagContextManager.calculateOptimalTokenAllocation(ragChunks, totalTokens)
  }

  return {
    // State
    state,
    selectedDocumentIds,
    
    // Computed
    hasDocuments,
    hasSelectedDocuments,
    readyDocuments,
    isProcessing,
    
    // Actions
    initialize,
    loadDocuments,
    uploadDocument,
    deleteDocument,
    searchDocuments,
    generateEmbeddings,
    loadSettings,
    updateSettings,
    loadStats,
    loadEmbeddingStatus,
    toggleDocumentSelection,
    selectAllDocuments,
    clearSelection,
    clearError,
    calculateTokenAllocation
  }
}