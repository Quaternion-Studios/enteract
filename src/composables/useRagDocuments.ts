// Backward compatibility adapter for legacy useRagDocuments API
import { computed, ref } from 'vue'
import { useRagSystem } from './rag/useRagSystem'

export function useRagDocuments() {
  const ragSystem = useRagSystem()
  
  // Constants for backward compatibility
  const CHAT_DOCUMENT_LIMIT = 5

  // Adapters for legacy API
  const documents = computed(() => ragSystem.state.documents)
  const selectedDocumentIds = computed(() => new Set(ragSystem.selectedDocumentIds.value))

  const initialize = async () => {
    await ragSystem.initialize()
  }

  const uploadDocuments = async (files: File[] | FileList, options?: { maxSelection?: number }) => {
    // Convert FileList to Array if needed
    const fileArray = Array.isArray(files) ? files : Array.from(files)
    const uploadedDocs = []
    
    for (const file of fileArray) {
      try {
        const doc = await ragSystem.uploadDocument(file)
        uploadedDocs.push(doc)
      } catch (error) {
        console.error(`Failed to upload ${file.name}:`, error)
        throw error
      }
    }
    
    return uploadedDocs
  }

  const toggleDocumentSelection = (documentId: string, context: string = 'chat') => {
    ragSystem.toggleDocumentSelection(documentId)
  }

  const clearSelection = () => {
    ragSystem.clearSelection()
  }

  const getSelectionLimitInfo = () => {
    return {
      current: ragSystem.selectedDocumentIds.value.length,
      max: CHAT_DOCUMENT_LIMIT,
      limit: CHAT_DOCUMENT_LIMIT,
      isAtLimit: ragSystem.selectedDocumentIds.value.length >= CHAT_DOCUMENT_LIMIT
    }
  }

  const deleteDocument = async (documentId: string) => {
    await ragSystem.deleteDocument(documentId)
  }

  const generateEmbeddings = async (documentIds?: string[]) => {
    await ragSystem.generateEmbeddings(documentIds)
  }

  const clearEmbeddingCache = async () => {
    // For now, this is a no-op since we're using simple TF-IDF
    console.log('Embedding cache cleared (TF-IDF based)')
  }

  const selectAllDocuments = () => {
    ragSystem.selectAllDocuments()
  }

  const getStorageStats = async () => {
    return ragSystem.loadStats()
  }

  const totalStorageSizeMB = computed(() => 
    ragSystem.state.stats?.database_size_mb || 0
  )

  const settings = computed(() => ragSystem.state.settings)

  // Additional computed properties for backward compatibility
  const cachedDocuments = computed(() => 
    ragSystem.state.documents.filter(doc => doc.is_cached)
  )

  const readyDocuments = computed(() => 
    ragSystem.state.documents.filter(doc => doc.embedding_status === 'completed')
  )

  const processingDocuments = computed(() => 
    ragSystem.state.documents.filter(doc => doc.embedding_status === 'processing')
  )

  return {
    // State
    documents,
    selectedDocumentIds: computed(() => new Set(ragSystem.selectedDocumentIds.value)),
    cachedDocuments,
    readyDocuments,
    processingDocuments,
    totalStorageSizeMB,
    settings,
    
    // Constants
    CHAT_DOCUMENT_LIMIT,
    
    // Methods
    initialize,
    uploadDocuments,
    toggleDocumentSelection,
    clearSelection,
    getSelectionLimitInfo,
    deleteDocument,
    generateEmbeddings,
    clearEmbeddingCache,
    selectAllDocuments,
    getStorageStats
  }
}