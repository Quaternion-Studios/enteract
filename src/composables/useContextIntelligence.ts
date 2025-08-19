import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRagDocuments } from './rag'
// import { useChatManagement } from './useChatManagement' // Removed to avoid circular dependencies
// import { useDocumentPriority } from './useDocumentPriority' // Removed to prevent circular dependency

export interface ContextDocument {
  id: string
  file_path: string
  filename: string
  relevance_score: number
  access_count: number
  last_accessed: Date
  content_preview?: string
  embedding_status: 'pending' | 'processing' | 'ready' | 'failed'
  metadata?: Record<string, any>
}

export interface ContextSession {
  id: string
  chat_id: string
  active_documents: string[]
  suggested_documents: string[]
  context_mode: ContextMode
  created_at: Date
  updated_at: Date
}

export type ContextMode = 'auto' | 'manual' | 'search' | 'all' | 'none'

export interface ContextSuggestion {
  document_id: string
  reason: string
  confidence: number
  relevant_chunks?: string[]
}

export interface ContextAnalysis {
  topics: string[]
  entities: string[]
  intent: string
  suggested_documents: ContextSuggestion[]
}

class ContextIntelligenceService {
  private contextCache = ref<Map<string, ContextDocument>>(new Map())
  private contextSession = ref<ContextSession | null>(null)
  private contextAnalysis = ref<ContextAnalysis | null>(null)
  private documentPriorityQueue = ref<string[]>([])
  private isProcessing = ref(false)
  
  // Configuration
  private readonly MAX_CACHED_DOCUMENTS = 10
  private readonly MIN_RELEVANCE_SCORE = 0.7
  private readonly CONTEXT_REFRESH_INTERVAL = 5000 // 5 seconds
  
  constructor() {
    this.initializeContextSystem()
  }
  
  private async initializeContextSystem() {
    try {
      // Load cached documents from backend
      await this.loadCachedDocuments()
      
      // Initialize context session
      await this.initializeSession()
      
      // Start background processing
      this.startBackgroundProcessing()
    } catch (error) {
      console.error('Failed to initialize context system:', error)
    }
  }
  
  async loadCachedDocuments() {
    try {
      console.log('🧠 Loading cached context documents...')
      // Load from backend
      const cached = await invoke<ContextDocument[]>('get_cached_context_documents')
      console.log('🧠 Loaded', cached.length, 'cached documents')
      
      // Sort by access count and relevance score
      const sortedDocs = cached.sort((a, b) => {
        const scoreA = a.access_count + a.relevance_score * 100
        const scoreB = b.access_count + b.relevance_score * 100
        return scoreB - scoreA
      })
      
      this.contextCache.value = new Map(sortedDocs.map(doc => [doc.id, doc]))
      console.log('🧠 Context cache initialized with', this.contextCache.value.size, 'documents')
    } catch (error) {
      console.error('🧠 Failed to load cached documents, continuing with empty cache:', error)
      // Initialize with empty cache to prevent undefined errors
      this.contextCache.value = new Map()
    }
  }
  
  async initializeSession() {
    try {
      // Use a default session ID for now, can be enhanced later to use actual chat IDs
      const chatId = `session_${Date.now()}`
      console.log('🧠 Initializing context session:', chatId)
      
      const session = await invoke<ContextSession>('initialize_context_session', {
        chatId
      })
      this.contextSession.value = session
      console.log('🧠 Context session initialized successfully:', session)
    } catch (error) {
      console.error('🧠 Failed to initialize context session, creating fallback:', error)
      // Create a fallback session to prevent undefined errors
      this.contextSession.value = {
        id: `fallback_${Date.now()}`,
        chat_id: `session_${Date.now()}`,
        active_documents: [],
        suggested_documents: [],
        context_mode: 'auto',
        created_at: new Date(),
        updated_at: new Date()
      }
    }
  }
  
  async analyzeConversation(messages: Array<{ role: string; content: string }>) {
    if (this.isProcessing.value) return
    
    this.isProcessing.value = true
    try {
      // Validate input messages
      if (!messages || !Array.isArray(messages) || messages.length === 0) {
        console.warn('🧠 No valid messages provided for context analysis')
        return null
      }
      
      const analysis = await invoke<ContextAnalysis>('analyze_conversation_context', {
        messages: messages.slice(-10) // Last 10 messages for context
      })
      
      // Validate the analysis response
      if (!analysis) {
        console.warn('🧠 No analysis returned from backend')
        return null
      }
      
      this.contextAnalysis.value = analysis
      
      // Update document suggestions with proper error handling
      if (analysis.suggested_documents && Array.isArray(analysis.suggested_documents)) {
        await this.updateDocumentSuggestions(analysis.suggested_documents)
      } else {
        console.warn('🧠 No valid document suggestions in analysis')
      }
      
      return analysis
    } catch (error) {
      console.error('🧠 Failed to analyze conversation:', error)
      // Set a minimal analysis to prevent undefined errors
      this.contextAnalysis.value = {
        topics: [],
        entities: [],
        intent: 'unknown',
        suggested_documents: []
      }
      return null
    } finally {
      this.isProcessing.value = false
    }
  }
  
  async updateDocumentSuggestions(suggestions: ContextSuggestion[]) {
    if (!this.contextSession.value || !suggestions || !Array.isArray(suggestions)) {
      console.warn('🧠 Invalid suggestions or no context session available')
      return
    }
    
    const documentIds = suggestions
      .filter(s => s && typeof s.confidence === 'number' && s.confidence >= this.MIN_RELEVANCE_SCORE)
      .sort((a, b) => b.confidence - a.confidence)
      .slice(0, this.MAX_CACHED_DOCUMENTS)
      .map(s => s.document_id)
      .filter(id => id) // Remove any undefined/null IDs
    
    this.contextSession.value.suggested_documents = documentIds
    
    // Update priority queue for background processing
    this.documentPriorityQueue.value = documentIds
    
    // Trigger background embedding processing
    if (documentIds.length > 0) {
      await this.processDocumentEmbeddings(documentIds)
    }
  }
  
  async processDocumentEmbeddings(documentIds: string[]) {
    const ragDocuments = useRagDocuments()
    
    for (const docId of documentIds) {
      const doc = this.contextCache.value.get(docId)
      if (doc && doc.embedding_status !== 'ready') {
        try {
          await invoke('process_document_embeddings', {
            documentId: docId,
            priority: 'high'
          })
          
          // Update embedding status
          doc.embedding_status = 'processing'
          this.contextCache.value.set(docId, doc)
        } catch (error) {
          console.error(`Failed to process embeddings for ${docId}:`, error)
        }
      }
    }
  }
  
  async selectDocuments(mode: ContextMode, query?: string): Promise<string[]> {
    console.log('🧠 Selecting documents with mode:', mode, 'query:', query)
    switch (mode) {
      case 'auto':
        return this.selectAutoDocuments()
      case 'manual':
        return [] // User will select manually
      case 'search':
        return query ? await this.searchDocuments(query) : []
      case 'all':
        const allDocs = Array.from(this.contextCache.value.keys())
        console.log('🧠 All mode: returning', allDocs.length, 'documents:', allDocs)
        
        // Fallback: if context cache is empty, try to get documents from RAG system directly
        if (allDocs.length === 0) {
          console.warn('🧠 Context cache is empty, trying to get documents from RAG system...')
          try {
            const ragDocuments = await invoke<any[]>('get_all_documents')
            const ragDocIds = ragDocuments.map(doc => doc.id)
            console.log('🧠 Fallback: using', ragDocIds.length, 'RAG documents:', ragDocIds)
            return ragDocIds
          } catch (error) {
            console.error('🧠 Failed to get RAG documents as fallback:', error)
            return []
          }
        }
        
        return allDocs
      case 'none':
        return []
      default:
        console.warn('🧠 Unknown context mode:', mode)
        return []
    }
  }
  
  private async selectAutoDocuments(): Promise<string[]> {
    // Add proper null/undefined checks
    if (!this.contextAnalysis.value || !this.contextAnalysis.value.suggested_documents) {
      console.warn('🧠 Context analysis not available, using cached documents as fallback')
      // Fallback to top cached documents by relevance
      return Array.from(this.contextCache.value.values())
        .filter(doc => doc.relevance_score >= this.MIN_RELEVANCE_SCORE)
        .sort((a, b) => (b.relevance_score + b.access_count) - (a.relevance_score + a.access_count))
        .slice(0, this.MAX_CACHED_DOCUMENTS)
        .map(doc => doc.id)
    }
    
    const suggestions = this.contextAnalysis.value.suggested_documents
    return suggestions
      .filter(s => s.confidence >= this.MIN_RELEVANCE_SCORE)
      .slice(0, this.MAX_CACHED_DOCUMENTS)
      .map(s => s.document_id)
  }
  
  async searchDocuments(query: string): Promise<string[]> {
    try {
      const results = await invoke<string[]>('search_context_documents', {
        query,
        limit: this.MAX_CACHED_DOCUMENTS
      })
      return results
    } catch (error) {
      console.error('Failed to search documents:', error)
      return []
    }
  }
  
  async addDocumentToContext(documentId: string) {
    if (!this.contextSession.value) return
    
    if (!this.contextSession.value.active_documents.includes(documentId)) {
      this.contextSession.value.active_documents.push(documentId)
      
      // Update access count and last accessed
      const doc = this.contextCache.value.get(documentId)
      if (doc) {
        doc.access_count++
        doc.last_accessed = new Date()
        this.contextCache.value.set(documentId, doc)
        
        // Update access in backend
        await invoke('update_document_access', {
          documentId,
          accessCount: doc.access_count,
          lastAccessed: doc.last_accessed.toISOString()
        })
      }
    }
  }
  
  async removeDocumentFromContext(documentId: string) {
    if (!this.contextSession.value) return
    
    const index = this.contextSession.value.active_documents.indexOf(documentId)
    if (index > -1) {
      this.contextSession.value.active_documents.splice(index, 1)
    }
  }
  
  getActiveDocuments(): ContextDocument[] {
    if (!this.contextSession.value) return []
    
    return this.contextSession.value.active_documents
      .map(id => this.contextCache.value.get(id))
      .filter((doc): doc is ContextDocument => doc !== undefined)
  }
  
  getSuggestedDocuments(): ContextDocument[] {
    if (!this.contextSession.value) return []
    
    return this.contextSession.value.suggested_documents
      .map(id => this.contextCache.value.get(id))
      .filter((doc): doc is ContextDocument => doc !== undefined)
  }
  
  async getContextForMessage(message: string): Promise<any> {
    const activeDocuments = this.getActiveDocuments()
    
    if (activeDocuments.length === 0) {
      return null
    }
    
    try {
      const context = await invoke('get_context_for_message', {
        message,
        documentIds: activeDocuments.map(d => d.id),
        maxChunks: 5
      })
      
      return context
    } catch (error) {
      console.error('Failed to get context for message:', error)
      return null
    }
  }
  
  private startBackgroundProcessing() {
    setInterval(async () => {
      if (this.documentPriorityQueue.value.length > 0 && !this.isProcessing.value) {
        const nextDocId = this.documentPriorityQueue.value.shift()
        if (nextDocId) {
          await this.processDocumentEmbeddings([nextDocId])
        }
      }
    }, this.CONTEXT_REFRESH_INTERVAL)
  }
  
  async setContextMode(mode: ContextMode) {
    if (this.contextSession.value) {
      this.contextSession.value.context_mode = mode
      
      await invoke('update_context_session', {
        sessionId: this.contextSession.value.id,
        mode
      })
    }
  }
  
  getContextMode(): ContextMode {
    return this.contextSession.value?.context_mode || 'none'
  }
  
  async clearContext() {
    if (this.contextSession.value) {
      this.contextSession.value.active_documents = []
      this.contextSession.value.suggested_documents = []
      this.contextAnalysis.value = null
    }
  }
  
  // Computed properties for reactive access
  isContextActive = computed(() => {
    return this.contextSession.value && 
           this.contextSession.value.active_documents.length > 0
  })
  
  contextDocumentCount = computed(() => {
    return this.contextSession.value?.active_documents.length || 0
  })
  
  hasSuggestions = computed(() => {
    return this.contextSession.value && 
           this.contextSession.value.suggested_documents.length > 0
  })
}

// Singleton instance
let contextIntelligenceInstance: ContextIntelligenceService | null = null

export function useContextIntelligence() {
  if (!contextIntelligenceInstance) {
    contextIntelligenceInstance = new ContextIntelligenceService()
  }
  
  return {
    analyzeConversation: contextIntelligenceInstance.analyzeConversation.bind(contextIntelligenceInstance),
    selectDocuments: contextIntelligenceInstance.selectDocuments.bind(contextIntelligenceInstance),
    addDocumentToContext: contextIntelligenceInstance.addDocumentToContext.bind(contextIntelligenceInstance),
    removeDocumentFromContext: contextIntelligenceInstance.removeDocumentFromContext.bind(contextIntelligenceInstance),
    getActiveDocuments: contextIntelligenceInstance.getActiveDocuments.bind(contextIntelligenceInstance),
    getSuggestedDocuments: contextIntelligenceInstance.getSuggestedDocuments.bind(contextIntelligenceInstance),
    getContextForMessage: contextIntelligenceInstance.getContextForMessage.bind(contextIntelligenceInstance),
    setContextMode: contextIntelligenceInstance.setContextMode.bind(contextIntelligenceInstance),
    getContextMode: contextIntelligenceInstance.getContextMode.bind(contextIntelligenceInstance),
    clearContext: contextIntelligenceInstance.clearContext.bind(contextIntelligenceInstance),
    isContextActive: contextIntelligenceInstance.isContextActive,
    contextDocumentCount: contextIntelligenceInstance.contextDocumentCount,
    hasSuggestions: contextIntelligenceInstance.hasSuggestions
  }
}