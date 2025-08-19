import { ref, computed, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface ContextSuggestion {
  document_id: string
  document_name: string
  relevance_score: number
  reason: string
  preview: string
}

export interface RankedDocument {
  document: any // Document type
  relevance_score: number
  rank_factors: Record<string, number>
}

export interface RelatedDocument {
  document_id: string
  document_name: string
  relationship_type: string
  similarity_score: number
}

export interface ConversationContext {
  topics: string[]
  entities: string[]
  keywords: string[]
  intent: string
}

export interface SearchFilters {
  file_types: string[]
  date_range?: {
    from: string
    to: string
  }
  tags: string[]
  min_size?: number
  max_size?: number
  has_embeddings?: boolean
}

export interface RankingOptions {
  semantic_weight: number
  keyword_weight: number
  recency_weight: number
  usage_weight: number
  boost_exact_matches: boolean
  penalize_duplicates: boolean
}

export function useContextIntelligence() {
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  
  // State
  const contextSuggestions = ref<ContextSuggestion[]>([])
  const searchResults = ref<RankedDocument[]>([])
  const relatedDocuments = ref<RelatedDocument[]>([])
  const conversationContext = ref<ConversationContext | null>(null)
  const documentAnalytics = ref<Record<string, any>>({})

  // Search state
  const currentQuery = ref('')
  const searchFilters = reactive<SearchFilters>({
    file_types: [],
    tags: [],
  })
  
  const rankingOptions = reactive<RankingOptions>({
    semantic_weight: 0.5,
    keyword_weight: 0.3,
    recency_weight: 0.1,
    usage_weight: 0.1,
    boost_exact_matches: true,
    penalize_duplicates: true,
  })

  // Computed
  const hasResults = computed(() => searchResults.value.length > 0)
  const totalDocuments = computed(() => searchResults.value.length)
  const topSuggestions = computed(() => 
    contextSuggestions.value.slice(0, 5)
  )

  /**
   * Search for documents across the entire collection with advanced ranking
   */
  const searchContextDocuments = async (query: string) => {
    if (!query.trim()) {
      searchResults.value = []
      return
    }

    isLoading.value = true
    error.value = null
    currentQuery.value = query

    try {
      const results = await invoke<RankedDocument[]>('search_context_documents', {
        query: query.trim(),
        limit: 50
      })
      
      searchResults.value = results
    } catch (err) {
      error.value = err as string
      console.error('Failed to search context documents:', err)
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Get AI-powered context suggestions based on conversation history
   */
  const getContextSuggestions = async (conversationHistory: string[]) => {
    if (!conversationHistory.length) {
      contextSuggestions.value = []
      return
    }

    try {
      const suggestions = await invoke<ContextSuggestion[]>('get_context_suggestions', {
        conversation_history: conversationHistory
      })
      
      contextSuggestions.value = suggestions
    } catch (err) {
      error.value = err as string
      console.error('Failed to get context suggestions:', err)
    }
  }

  /**
   * Find documents related to the currently selected documents
   */
  const getRelatedDocuments = async (documentIds: string[]) => {
    if (!documentIds.length) {
      relatedDocuments.value = []
      return
    }

    try {
      const related = await invoke<RelatedDocument[]>('get_related_documents', {
        document_ids: documentIds
      })
      
      relatedDocuments.value = related
    } catch (err) {
      error.value = err as string
      console.error('Failed to get related documents:', err)
    }
  }

  /**
   * Analyze conversation context to extract topics, keywords, and intent
   */
  const analyzeConversationContext = async (messages: string[]) => {
    if (!messages.length) {
      conversationContext.value = null
      return
    }

    try {
      const formatted = messages.map(m => ({ role: 'user', content: m }))
      const context = await invoke<Record<string, any>>('analyze_conversation_context', {
        messages: formatted
      })
      
      conversationContext.value = {
        topics: context.topics || [],
        entities: context.entities || [],
        keywords: context.keywords || [],
        intent: context.intent || 'general'
      }
    } catch (err) {
      error.value = err as string
      console.error('Failed to analyze conversation context:', err)
    }
  }

  /**
   * Get document usage analytics and insights
   */
  const getDocumentAnalytics = async () => {
    try {
      const analytics = await invoke<Record<string, any>>('get_document_analytics')
      documentAnalytics.value = analytics
    } catch (err) {
      error.value = err as string
      console.error('Failed to get document analytics:', err)
    }
  }

  /**
   * Clean up orphaned documents whose files no longer exist
   */
  const cleanupOrphanedDocuments = async () => {
    try {
      const cleanedUp = await invoke<string[]>('cleanup_orphaned_documents')
      console.log(`Cleaned up ${cleanedUp.length} orphaned documents`)
      return cleanedUp
    } catch (err) {
      error.value = err as string
      console.error('Failed to cleanup orphaned documents:', err)
      return []
    }
  }

  /**
   * Suggest context for a specific message based on AI analysis
   */
  const suggestContextForMessage = async (message: string, history: string[] = []) => {
    const allMessages = [...history, message]
    await Promise.all([
      getContextSuggestions(allMessages),
      analyzeConversationContext(allMessages)
    ])
  }

  /**
   * Perform intelligent context search with filters and ranking
   */
  const performIntelligentSearch = async (
    query: string, 
    filters?: Partial<SearchFilters>,
    ranking?: Partial<RankingOptions>
  ) => {
    // Update filters and ranking if provided
    if (filters) {
      Object.assign(searchFilters, filters)
    }
    if (ranking) {
      Object.assign(rankingOptions, ranking)
    }

    // Perform the search
    await searchContextDocuments(query)
  }

  /**
   * Auto-suggest relevant documents based on current conversation context
   */
  const autoSuggestRelevantDocuments = async (
    currentMessage: string,
    conversationHistory: string[] = [],
    selectedDocuments: string[] = []
  ) => {
    // Analyze the current context
    await analyzeConversationContext([...conversationHistory, currentMessage])
    
    // Get context suggestions
    await getContextSuggestions([...conversationHistory, currentMessage])
    
    // Get related documents if any are selected
    if (selectedDocuments.length > 0) {
      await getRelatedDocuments(selectedDocuments)
    }
  }

  /**
   * Clear all context state
   */
  const clearContext = () => {
    contextSuggestions.value = []
    searchResults.value = []
    relatedDocuments.value = []
    conversationContext.value = null
    currentQuery.value = ''
    error.value = null
  }

  /**
   * Reset search filters to defaults
   */
  const resetFilters = () => {
    searchFilters.file_types = []
    searchFilters.tags = []
    searchFilters.date_range = undefined
    searchFilters.min_size = undefined
    searchFilters.max_size = undefined
    searchFilters.has_embeddings = undefined
  }

  /**
   * Get search suggestions for autocomplete
   */
  const getSearchSuggestions = async (partialQuery: string) => {
    if (partialQuery.length < 2) return []
    
    // Simple client-side suggestions for now
    const suggestions = [
      'authentication methods',
      'database configuration', 
      'API documentation',
      'testing strategies',
      'deployment guide',
      'security best practices',
      'performance optimization',
      'error handling',
      'user interface design',
      'data validation'
    ]
    
    return suggestions.filter(s => 
      s.toLowerCase().includes(partialQuery.toLowerCase())
    ).slice(0, 5)
  }

  return {
    // State
    isLoading,
    error,
    contextSuggestions,
    searchResults,
    relatedDocuments,
    conversationContext,
    documentAnalytics,
    currentQuery,
    searchFilters,
    rankingOptions,

    // Computed
    hasResults,
    totalDocuments,
    topSuggestions,

    // Actions
    searchContextDocuments,
    getContextSuggestions,
    getRelatedDocuments,
    analyzeConversationContext,
    getDocumentAnalytics,
    cleanupOrphanedDocuments,
    suggestContextForMessage,
    performIntelligentSearch,
    autoSuggestRelevantDocuments,
    clearContext,
    resetFilters,
    getSearchSuggestions,
  }
}