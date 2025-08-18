import { ref, computed } from 'vue'
import { ragService, type Document, type DocumentChunk, type RagSettings } from '../services/ragService'
import { enhancedRagService, type EnhancedDocument, type EnhancedDocumentChunk, type EnhancedRagSettings } from '../services/enhancedRagService'

export interface UploadContext {
  source: 'chat' | 'settings'
  autoSelect?: boolean
  maxSelection?: number
}

export function useRagDocuments() {
  // State - Using enhanced types but keeping backward compatibility
  const documents = ref<EnhancedDocument[]>([])
  const selectedDocumentIds = ref<Set<string>>(new Set())
  const sessionSelectedDocuments = ref<Map<string, Set<string>>>(new Map()) // Per-session selection
  const currentSessionId = ref<string | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const uploadProgress = ref<Map<string, number>>(new Map())
  const settings = ref<EnhancedRagSettings | null>(null)
  const searchResults = ref<EnhancedDocumentChunk[]>([])
  const isSearching = ref(false)
  const useEnhanced = ref(true) // Flag to enable enhanced RAG system
  const embeddingStatus = ref<Map<string, string>>(new Map())
  
  // Chat-specific document limit
  const CHAT_DOCUMENT_LIMIT = 5

  // Session Management
  const initializeSession = (sessionId: string) => {
    currentSessionId.value = sessionId
    if (!sessionSelectedDocuments.value.has(sessionId)) {
      sessionSelectedDocuments.value.set(sessionId, new Set())
    }
    console.log(`📂 Initialized RAG session: ${sessionId}`)
  }

  const clearSession = (sessionId?: string) => {
    const targetSessionId = sessionId || currentSessionId.value
    if (targetSessionId) {
      sessionSelectedDocuments.value.delete(targetSessionId)
      if (currentSessionId.value === targetSessionId) {
        currentSessionId.value = null
      }
      console.log(`🗑️ Cleared RAG session: ${targetSessionId}`)
    }
  }

  const getActiveSelection = (): Set<string> => {
    if (currentSessionId.value && sessionSelectedDocuments.value.has(currentSessionId.value)) {
      return sessionSelectedDocuments.value.get(currentSessionId.value) || new Set()
    }
    return selectedDocumentIds.value // Fallback to global selection
  }

  const setActiveSelection = (selection: Set<string>) => {
    if (currentSessionId.value) {
      sessionSelectedDocuments.value.set(currentSessionId.value, new Set(selection))
    } else {
      selectedDocumentIds.value = new Set(selection)
      saveSelectedDocuments()
    }
  }

  // Computed
  const selectedDocuments = computed(() => {
    const activeSelection = getActiveSelection()
    return documents.value.filter(doc => activeSelection.has(doc.id))
  })

  const cachedDocuments = computed(() => {
    return documents.value.filter(doc => doc.is_cached)
  })

  const totalStorageSize = computed(() => {
    return documents.value.reduce((sum, doc) => sum + doc.file_size, 0)
  })

  const totalStorageSizeMB = computed(() => {
    return totalStorageSize.value / (1024 * 1024)
  })

  // Initialize RAG system
  const initialize = async () => {
    try {
      isLoading.value = true
      error.value = null
      
      if (useEnhanced.value) {
        await enhancedRagService.initialize()
      } else {
        await ragService.initialize()
      }
      await loadDocuments()
      await loadSettings()
      
      console.log('RAG system initialized with documents:', documents.value.length)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to initialize RAG system'
      console.error('Failed to initialize RAG system:', err)
    } finally {
      isLoading.value = false
    }
  }

  // Load all documents
  const loadDocuments = async () => {
    try {
      isLoading.value = true
      error.value = null
      
      const docs = useEnhanced.value 
        ? await enhancedRagService.getAllDocuments()
        : await ragService.getAllDocuments() as EnhancedDocument[]
      documents.value = docs
      
      // Restore selected documents from localStorage (global fallback)
      const savedSelection = localStorage.getItem('rag_selected_documents')
      if (savedSelection) {
        const savedIds = JSON.parse(savedSelection) as string[]
        selectedDocumentIds.value = new Set(savedIds.filter(id => 
          documents.value.some(doc => doc.id === id)
        ))
      }
      
      // Load embedding status for all documents
      await updateEmbeddingStatus()
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to load documents'
      console.error('Failed to load documents:', err)
    } finally {
      isLoading.value = false
    }
  }

  // Load settings
  const loadSettings = async () => {
    try {
      settings.value = useEnhanced.value 
        ? await enhancedRagService.getSettings()
        : await ragService.getSettings() as EnhancedRagSettings
    } catch (err) {
      console.error('Failed to load RAG settings:', err)
    }
  }

  // Upload document with progress tracking and context
  const uploadDocument = async (file: File, context?: UploadContext): Promise<EnhancedDocument | null> => {
    try {
      error.value = null
      
      // Validate file
      if (useEnhanced.value) {
        const validation = await enhancedRagService.validateFileUpload(file)
        if (!validation.valid) {
          error.value = validation.error || 'File validation failed'
          return null
        }
        
        // Check for duplicates in enhanced system
        const duplicateCheck = await enhancedRagService.checkDocumentDuplicate(file)
        if (duplicateCheck.isDuplicate && duplicateCheck.existingDocument) {
          console.info(`Document "${file.name}" already exists, using existing version`)
          
          // Add existing document to the list if not already there
          const existingInList = documents.value.find(d => d.id === duplicateCheck.existingDocument!.id)
          if (!existingInList) {
            documents.value.unshift(duplicateCheck.existingDocument)
          }
          
          // Handle context-specific behavior for existing document
          const uploadContext = context || { source: 'settings', autoSelect: false }
          if (uploadContext.source === 'chat') {
            // Auto-select the existing document in chat context
            const maxDocs = uploadContext.maxSelection || CHAT_DOCUMENT_LIMIT
            if (selectedDocumentIds.value.size >= maxDocs) {
              const sortedSelected = Array.from(selectedDocumentIds.value)
                .map(id => documents.value.find(d => d.id === id))
                .filter(Boolean)
                .sort((a, b) => {
                  const aTime = new Date(a!.created_at).getTime()
                  const bTime = new Date(b!.created_at).getTime()
                  return aTime - bTime
                })
              
              if (sortedSelected.length > 0) {
                selectedDocumentIds.value.delete(sortedSelected[0]!.id)
              }
            }
            selectedDocumentIds.value.add(duplicateCheck.existingDocument.id)
            saveSelectedDocuments()
          }
          
          return duplicateCheck.existingDocument
        }
      } else if (settings.value) {
        const validation = ragService.validateFile(file, settings.value as RagSettings)
        if (!validation.valid) {
          error.value = validation.error || 'File validation failed'
          return null
        }
      }
      
      // Track upload progress
      const fileId = `${file.name}-${Date.now()}`
      uploadProgress.value.set(fileId, 0)
      
      // Simulate progress updates (real progress would come from backend)
      const progressInterval = setInterval(() => {
        const current = uploadProgress.value.get(fileId) || 0
        if (current < 90) {
          uploadProgress.value.set(fileId, current + 10)
        }
      }, 200)
      
      // Upload document
      const document = useEnhanced.value 
        ? await enhancedRagService.uploadDocument(file)
        : await ragService.uploadDocument(file) as EnhancedDocument
      
      // Complete progress
      clearInterval(progressInterval)
      uploadProgress.value.set(fileId, 100)
      
      // Add to documents list
      documents.value.unshift(document)
      
      // Handle context-specific behavior
      const uploadContext = context || { source: 'settings', autoSelect: false }
      
      if (uploadContext.source === 'chat') {
        // Chat context: Always auto-select but enforce limit
        const maxDocs = uploadContext.maxSelection || CHAT_DOCUMENT_LIMIT
        
        // If we're at the limit, deselect the oldest document
        if (selectedDocumentIds.value.size >= maxDocs) {
          const sortedSelected = Array.from(selectedDocumentIds.value)
            .map(id => documents.value.find(d => d.id === id))
            .filter(Boolean)
            .sort((a, b) => {
              const aTime = new Date(a!.created_at).getTime()
              const bTime = new Date(b!.created_at).getTime()
              return aTime - bTime
            })
          
          if (sortedSelected.length > 0) {
            selectedDocumentIds.value.delete(sortedSelected[0]!.id)
          }
        }
        
        selectedDocumentIds.value.add(document.id)
      } else if (uploadContext.autoSelect !== false && uploadContext.source === 'settings') {
        // Settings context: Only auto-select if explicitly requested
        // By default, don't auto-select in settings
      }
      
      saveSelectedDocuments()
      
      // Clear progress after delay
      setTimeout(() => {
        uploadProgress.value.delete(fileId)
      }, 2000)
      
      return document
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to upload document'
      console.error('Failed to upload document:', err)
      return null
    }
  }

  // Upload multiple documents with context
  const uploadDocuments = async (files: FileList | File[], context?: UploadContext): Promise<EnhancedDocument[]> => {
    const uploaded: EnhancedDocument[] = []
    
    for (const file of files) {
      const doc = await uploadDocument(file, context)
      if (doc) {
        uploaded.push(doc)
      }
    }
    
    return uploaded
  }

  // Delete document
  const deleteDocument = async (documentId: string) => {
    try {
      error.value = null
      
      if (useEnhanced.value) {
        await enhancedRagService.deleteDocument(documentId)
      } else {
        await ragService.deleteDocument(documentId)
      }
      
      // Remove from local state
      documents.value = documents.value.filter(doc => doc.id !== documentId)
      selectedDocumentIds.value.delete(documentId)
      saveSelectedDocuments()
      
      console.log(`Document ${documentId} deleted`)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to delete document'
      console.error('Failed to delete document:', err)
    }
  }

  // Check if selection limit is reached for chat context
  const isSelectionLimitReached = (context: 'chat' | 'settings' = 'settings'): boolean => {
    if (context === 'chat') {
      return selectedDocumentIds.value.size >= CHAT_DOCUMENT_LIMIT
    }
    return false
  }
  
  // Get selection limit info
  const getSelectionLimitInfo = () => ({
    current: selectedDocumentIds.value.size,
    max: CHAT_DOCUMENT_LIMIT,
    isAtLimit: selectedDocumentIds.value.size >= CHAT_DOCUMENT_LIMIT
  })
  
  // Toggle document selection with context awareness
  const toggleDocumentSelection = (documentId: string, context: 'chat' | 'settings' = 'settings') => {
    const activeSelection = getActiveSelection()
    const newSelection = new Set(activeSelection)
    
    if (newSelection.has(documentId)) {
      newSelection.delete(documentId)
    } else {
      // Check limit based on context
      if (context === 'chat') {
        // Enforce chat document limit
        if (newSelection.size >= CHAT_DOCUMENT_LIMIT) {
          error.value = `Maximum ${CHAT_DOCUMENT_LIMIT} documents can be selected in chat`
          return
        }
      } else if (settings.value && newSelection.size >= settings.value.max_cached_documents) {
        // Settings context: Check cache limit
        const oldestId = Array.from(newSelection)[0]
        newSelection.delete(oldestId)
      }
      newSelection.add(documentId)
      
      // Trigger priority embedding for newly selected document
      ensureDocumentEmbeddings([documentId])
    }
    
    setActiveSelection(newSelection)
  }

  // Select all documents
  const selectAllDocuments = () => {
    documents.value.forEach(doc => {
      if (selectedDocumentIds.value.size < (settings.value?.max_cached_documents || 5)) {
        selectedDocumentIds.value.add(doc.id)
      }
    })
    saveSelectedDocuments()
  }

  // Clear selection
  const clearSelection = () => {
    selectedDocumentIds.value.clear()
    saveSelectedDocuments()
  }

  // Save selected documents to localStorage
  const saveSelectedDocuments = () => {
    const selectedIds = Array.from(selectedDocumentIds.value)
    localStorage.setItem('rag_selected_documents', JSON.stringify(selectedIds))
  }

  // Enhanced embedding status management
  const updateEmbeddingStatus = async () => {
    if (!useEnhanced.value) return
    
    try {
      const docIds = documents.value.map(doc => doc.id)
      if (docIds.length === 0) return
      
      const statusMap = await enhancedRagService.getDocumentEmbeddingStatus(docIds)
      embeddingStatus.value = new Map(Object.entries(statusMap))
    } catch (err) {
      console.error('Failed to update embedding status:', err)
    }
  }

  const ensureDocumentEmbeddings = async (documentIds: string[]) => {
    if (!useEnhanced.value || documentIds.length === 0) return
    
    try {
      await enhancedRagService.generateEmbeddingsForSelection(documentIds)
      console.log(`🔄 Triggered priority embedding generation for ${documentIds.length} documents`)
      
      // Update status after triggering embeddings
      setTimeout(updateEmbeddingStatus, 1000)
    } catch (err) {
      console.error('Failed to ensure document embeddings:', err)
    }
  }

  const ensureSelectionReady = async () => {
    const activeSelection = getActiveSelection()
    const selectedIds = Array.from(activeSelection)
    
    if (selectedIds.length === 0) return { ready: [], pending: [] }
    
    try {
      const readinessMap = await enhancedRagService.ensureDocumentsReadyForSearch(selectedIds)
      
      const ready = selectedIds.filter(id => readinessMap[id] === 'ready')
      const pending = selectedIds.filter(id => 
        ['embedding_queued', 'embedding_processing', 'embedding_retry_queued'].includes(readinessMap[id])
      )
      
      return { ready, pending, status: readinessMap }
    } catch (err) {
      console.error('Failed to ensure selection readiness:', err)
      return { ready: [], pending: selectedIds }
    }
  }

  // Search documents with enhanced readiness checking
  const searchDocuments = async (query: string, useSelectedOnly = true) => {
    try {
      isSearching.value = true
      error.value = null
      
      const activeSelection = getActiveSelection()
      const contextIds = useSelectedOnly ? Array.from(activeSelection) : []
      
      // Ensure selected documents are ready for search
      if (contextIds.length > 0 && useEnhanced.value) {
        const readiness = await ensureSelectionReady()
        console.log(`📊 Document readiness: ${readiness.ready.length} ready, ${readiness.pending.length} pending`)
        
        if (readiness.ready.length === 0 && readiness.pending.length > 0) {
          error.value = `Documents are still processing embeddings. Please wait a moment and try again.`
          return []
        }
      }
      
      searchResults.value = useEnhanced.value 
        ? await enhancedRagService.searchDocuments(query, contextIds)
        : await ragService.searchDocuments(query, contextIds) as EnhancedDocumentChunk[]
      
      return searchResults.value
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Search failed'
      console.error('Document search failed:', err)
      return []
    } finally {
      isSearching.value = false
    }
  }

  // Get document by ID
  const getDocumentById = (documentId: string): EnhancedDocument | undefined => {
    return documents.value.find(doc => doc.id === documentId)
  }

  // Generate embeddings for a document
  const generateEmbeddings = async (documentId: string) => {
    try {
      if (useEnhanced.value) {
        await enhancedRagService.generateEmbeddings(documentId)
      } else {
        await ragService.generateEmbeddings(documentId)
      }
      
      // Update document cache status
      const doc = documents.value.find(d => d.id === documentId)
      if (doc) {
        doc.is_cached = true
        if ('embedding_status' in doc) {
          doc.embedding_status = 'processing'
        }
      }
    } catch (err) {
      console.error('Failed to generate embeddings:', err)
    }
  }

  // Clear embedding cache
  const clearEmbeddingCache = async () => {
    try {
      if (useEnhanced.value) {
        await enhancedRagService.clearEmbeddingCache()
      } else {
        await ragService.clearEmbeddingCache()
      }
      
      // Update all documents cache status
      documents.value.forEach(doc => {
        doc.is_cached = false
        if ('embedding_status' in doc) {
          doc.embedding_status = 'pending'
        }
      })
    } catch (err) {
      console.error('Failed to clear embedding cache:', err)
    }
  }

  // Update settings
  const updateSettings = async (newSettings: Partial<EnhancedRagSettings>) => {
    try {
      if (!settings.value) return
      
      const updatedSettings = { ...settings.value, ...newSettings }
      if (useEnhanced.value) {
        await enhancedRagService.updateSettings(updatedSettings)
      } else {
        await ragService.updateSettings(updatedSettings as RagSettings)
      }
      settings.value = updatedSettings
    } catch (err) {
      console.error('Failed to update settings:', err)
    }
  }

  // Get storage statistics
  const getStorageStats = async () => {
    try {
      return useEnhanced.value 
        ? await enhancedRagService.getStorageStats()
        : await ragService.getStorageStats()
    } catch (err) {
      console.error('Failed to get storage stats:', err)
      return null
    }
  }

  // Format document context for AI
  const formatContextForAI = (chunks: EnhancedDocumentChunk[]): string => {
    return useEnhanced.value 
      ? enhancedRagService.formatContextForAI(chunks)
      : formatLegacyContextForAI(chunks)
  }

  // Legacy formatting for backward compatibility
  const formatLegacyContextForAI = (chunks: EnhancedDocumentChunk[]): string => {
    if (chunks.length === 0) return ''
    
    const grouped = chunks.reduce((acc, chunk) => {
      if (!acc[chunk.document_id]) {
        acc[chunk.document_id] = []
      }
      acc[chunk.document_id].push(chunk)
      return acc
    }, {} as Record<string, EnhancedDocumentChunk[]>)
    
    let context = 'Relevant document context:\n\n'
    
    for (const [docId, docChunks] of Object.entries(grouped)) {
      const doc = getDocumentById(docId)
      if (doc) {
        context += `From "${doc.file_name}":\n`
        docChunks.forEach(chunk => {
          context += `- ${chunk.content.trim()}\n`
        })
        context += '\n'
      }
    }
    
    return context
  }

  // Enhanced methods
  const getEmbeddingStatus = async () => {
    if (useEnhanced.value) {
      try {
        return await enhancedRagService.getEmbeddingStatus()
      } catch (err) {
        console.error('Failed to get embedding status:', err)
        return null
      }
    }
    return null
  }

  const validateFile = async (file: File) => {
    if (useEnhanced.value) {
      try {
        return await enhancedRagService.validateFileUpload(file)
      } catch (err) {
        console.error('Failed to validate file:', err)
        return { valid: false, error: 'Validation failed' }
      }
    }
    
    // Legacy validation
    if (settings.value) {
      return ragService.validateFile(file, settings.value as RagSettings)
    }
    return { valid: true }
  }

  return {
    // State
    documents,
    selectedDocumentIds,
    selectedDocuments,
    cachedDocuments,
    isLoading,
    error,
    uploadProgress,
    settings,
    searchResults,
    isSearching,
    totalStorageSize,
    totalStorageSizeMB,
    useEnhanced,
    embeddingStatus,
    currentSessionId,
    
    // Methods
    initialize,
    loadDocuments,
    loadSettings,
    uploadDocument,
    uploadDocuments,
    deleteDocument,
    toggleDocumentSelection,
    selectAllDocuments,
    clearSelection,
    searchDocuments,
    getDocumentById,
    generateEmbeddings,
    clearEmbeddingCache,
    updateSettings,
    getStorageStats,
    formatContextForAI,
    isSelectionLimitReached,
    getSelectionLimitInfo,
    
    // Enhanced methods
    getEmbeddingStatus,
    validateFile,
    updateEmbeddingStatus,
    ensureDocumentEmbeddings,
    ensureSelectionReady,
    
    // Session management
    initializeSession,
    clearSession,
    getActiveSelection,
    setActiveSelection,
    
    // Constants
    CHAT_DOCUMENT_LIMIT,
    
    // Service references for advanced usage
    ragService,
    enhancedRagService
  }
}