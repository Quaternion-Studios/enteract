import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface DocumentPriority {
  documentId: string
  priority: number
  reason: string
  lastUpdated: Date
  factors: PriorityFactors
}

export interface PriorityFactors {
  accessFrequency: number
  recency: number
  contextRelevance: number
  embeddingStatus: number
  fileSize: number
  userPreference: number
}

export interface CacheStrategy {
  maxCachedDocuments: number
  priorityThreshold: number
  backgroundProcessing: boolean
  preloadSimilarDocuments: boolean
}

class DocumentPriorityService {
  private documentPriorities = ref<Map<string, DocumentPriority>>(new Map())
  private cacheStrategy = ref<CacheStrategy>({
    maxCachedDocuments: 10,
    priorityThreshold: 0.7,
    backgroundProcessing: true,
    preloadSimilarDocuments: true,
  })
  private isProcessing = ref(false)
  
  // Priority calculation weights
  private readonly WEIGHTS = {
    ACCESS_FREQUENCY: 0.25,
    RECENCY: 0.20,
    CONTEXT_RELEVANCE: 0.30,
    EMBEDDING_STATUS: 0.15,
    FILE_SIZE: -0.05, // Negative weight for large files
    USER_PREFERENCE: 0.15,
  }
  
  constructor() {
    this.initializePriorityService()
  }
  
  private async initializePriorityService() {
    try {
      // Load existing priorities from backend
      await this.loadDocumentPriorities()
      
      // Start background priority updates
      if (this.cacheStrategy.value.backgroundProcessing) {
        this.startBackgroundProcessing()
      }
    } catch (error) {
      console.error('Failed to initialize document priority service:', error)
    }
  }
  
  async loadDocumentPriorities() {
    try {
      const priorities = await invoke<DocumentPriority[]>('get_document_priorities')
      this.documentPriorities.value = new Map(
        priorities.map(p => [p.documentId, p])
      )
    } catch (error) {
      console.error('Failed to load document priorities:', error)
    }
  }
  
  async calculateDocumentPriority(
    documentId: string,
    accessCount: number = 0,
    lastAccessed: Date = new Date(),
    contextRelevance: number = 0,
    embeddingReady: boolean = false,
    fileSizeBytes: number = 0,
    userPreference: number = 0,
  ): Promise<DocumentPriority> {
    const now = new Date()
    
    // Calculate individual factors (0-1 scale)
    const factors: PriorityFactors = {
      // Access frequency: logarithmic scale up to 100 accesses
      accessFrequency: Math.min(Math.log10(accessCount + 1) / 2, 1),
      
      // Recency: exponential decay over 30 days
      recency: Math.exp(-(now.getTime() - lastAccessed.getTime()) / (30 * 24 * 60 * 60 * 1000)),
      
      // Context relevance: passed directly
      contextRelevance,
      
      // Embedding status: binary factor
      embeddingStatus: embeddingReady ? 1 : 0,
      
      // File size: inverse relationship (0-1 for files up to 10MB)
      fileSize: Math.max(0, 1 - (fileSizeBytes / (10 * 1024 * 1024))),
      
      // User preference: passed directly
      userPreference,
    }
    
    // Calculate weighted priority score
    const priority = 
      factors.accessFrequency * this.WEIGHTS.ACCESS_FREQUENCY +
      factors.recency * this.WEIGHTS.RECENCY +
      factors.contextRelevance * this.WEIGHTS.CONTEXT_RELEVANCE +
      factors.embeddingStatus * this.WEIGHTS.EMBEDDING_STATUS +
      factors.fileSize * this.WEIGHTS.FILE_SIZE +
      factors.userPreference * this.WEIGHTS.USER_PREFERENCE
    
    // Determine priority reason
    const maxFactor = Math.max(
      factors.accessFrequency,
      factors.recency,
      factors.contextRelevance,
      factors.userPreference
    )
    
    let reason = 'Low priority'
    if (maxFactor === factors.contextRelevance && factors.contextRelevance > 0.7) {
      reason = 'High context relevance'
    } else if (maxFactor === factors.accessFrequency && factors.accessFrequency > 0.6) {
      reason = 'Frequently accessed'
    } else if (maxFactor === factors.recency && factors.recency > 0.8) {
      reason = 'Recently accessed'
    } else if (maxFactor === factors.userPreference && factors.userPreference > 0.7) {
      reason = 'User preference'
    } else if (priority > this.cacheStrategy.value.priorityThreshold) {
      reason = 'Multiple factors'
    }
    
    const documentPriority: DocumentPriority = {
      documentId,
      priority: Math.max(0, Math.min(1, priority)), // Clamp between 0-1
      reason,
      lastUpdated: now,
      factors,
    }
    
    // Store in cache
    this.documentPriorities.value.set(documentId, documentPriority)
    
    // Persist to backend
    await this.persistDocumentPriority(documentPriority)
    
    return documentPriority
  }
  
  async updateDocumentAccess(documentId: string, contextRelevance?: number) {
    try {
      // Get current document info
      const docInfo = await invoke<any>('get_document_info', { documentId })
      
      // Calculate new priority
      const priority = await this.calculateDocumentPriority(
        documentId,
        (docInfo.access_count || 0) + 1,
        new Date(),
        contextRelevance || 0,
        docInfo.embedding_status === 'completed',
        docInfo.file_size || 0,
        docInfo.user_preference || 0,
      )
      
      // Update access in backend
      await invoke('update_document_access', {
        documentId,
        accessCount: (docInfo.access_count || 0) + 1,
        lastAccessed: new Date().toISOString(),
      })
      
      return priority
    } catch (error) {
      console.error('Failed to update document access:', error)
      return null
    }
  }
  
  async getTopPriorityDocuments(limit: number = 10): Promise<DocumentPriority[]> {
    const priorities = Array.from(this.documentPriorities.value.values())
    
    return priorities
      .filter(p => p.priority >= this.cacheStrategy.value.priorityThreshold)
      .sort((a, b) => b.priority - a.priority)
      .slice(0, limit)
  }
  
  async getCachedDocuments(): Promise<string[]> {
    const topPriorities = await this.getTopPriorityDocuments(
      this.cacheStrategy.value.maxCachedDocuments
    )
    
    return topPriorities.map(p => p.documentId)
  }
  
  async shouldCacheDocument(documentId: string): Promise<boolean> {
    const priority = this.documentPriorities.value.get(documentId)
    
    if (!priority) {
      return false
    }
    
    return priority.priority >= this.cacheStrategy.value.priorityThreshold
  }
  
  async preloadSimilarDocuments(documentId: string): Promise<string[]> {
    if (!this.cacheStrategy.value.preloadSimilarDocuments) {
      return []
    }
    
    try {
      // Get similar documents from backend
      const similarDocs = await invoke<string[]>('get_similar_documents', {
        documentId,
        limit: 3,
      })
      
      // Calculate priorities for similar documents
      for (const simDocId of similarDocs) {
        await this.updateDocumentAccess(simDocId, 0.6) // Medium relevance
      }
      
      return similarDocs
    } catch (error) {
      console.error('Failed to preload similar documents:', error)
      return []
    }
  }
  
  async evictLowPriorityDocuments() {
    const allPriorities = Array.from(this.documentPriorities.value.values())
    const sortedByPriority = allPriorities.sort((a, b) => a.priority - b.priority)
    
    const maxCached = this.cacheStrategy.value.maxCachedDocuments
    const toEvict = sortedByPriority.slice(0, Math.max(0, allPriorities.length - maxCached))
    
    for (const priority of toEvict) {
      try {
        await invoke('evict_document_from_cache', {
          documentId: priority.documentId,
        })
        
        console.log(`Evicted low-priority document: ${priority.documentId} (priority: ${priority.priority})`)
      } catch (error) {
        console.error(`Failed to evict document ${priority.documentId}:`, error)
      }
    }
  }
  
  private async persistDocumentPriority(priority: DocumentPriority) {
    try {
      await invoke('save_document_priority', {
        priority: {
          document_id: priority.documentId,
          priority: priority.priority,
          reason: priority.reason,
          last_updated: priority.lastUpdated.toISOString(),
          factors: priority.factors,
        },
      })
    } catch (error) {
      console.error('Failed to persist document priority:', error)
    }
  }
  
  private startBackgroundProcessing() {
    // Update priorities every 5 minutes
    setInterval(async () => {
      if (this.isProcessing.value) return
      
      this.isProcessing.value = true
      try {
        // Recalculate priorities for all documents
        await this.recalculateAllPriorities()
        
        // Evict low-priority documents
        await this.evictLowPriorityDocuments()
        
        // Preload high-priority documents
        await this.preloadHighPriorityDocuments()
      } catch (error) {
        console.error('Background processing failed:', error)
      } finally {
        this.isProcessing.value = false
      }
    }, 5 * 60 * 1000) // 5 minutes
  }
  
  private async recalculateAllPriorities() {
    try {
      const allDocuments = await invoke<any[]>('get_all_documents')
      
      for (const doc of allDocuments) {
        await this.calculateDocumentPriority(
          doc.id,
          doc.access_count || 0,
          doc.last_accessed ? new Date(doc.last_accessed) : new Date(),
          0, // Context relevance needs to be calculated separately
          doc.embedding_status === 'completed',
          doc.file_size || 0,
          doc.user_preference || 0,
        )
      }
    } catch (error) {
      console.error('Failed to recalculate priorities:', error)
    }
  }
  
  private async preloadHighPriorityDocuments() {
    const topPriorities = await this.getTopPriorityDocuments(
      this.cacheStrategy.value.maxCachedDocuments
    )
    
    for (const priority of topPriorities) {
      try {
        await invoke('ensure_document_cached', {
          documentId: priority.documentId,
        })
      } catch (error) {
        console.error(`Failed to cache document ${priority.documentId}:`, error)
      }
    }
  }
  
  async updateCacheStrategy(strategy: Partial<CacheStrategy>) {
    this.cacheStrategy.value = { ...this.cacheStrategy.value, ...strategy }
    
    // Persist new strategy
    try {
      await invoke('update_cache_strategy', {
        strategy: this.cacheStrategy.value,
      })
    } catch (error) {
      console.error('Failed to update cache strategy:', error)
    }
  }
  
  // Computed properties
  get isProcessingPriorities() {
    return this.isProcessing.value
  }
  
  get currentCacheStrategy() {
    return this.cacheStrategy.value
  }
  
  get priorityCount() {
    return this.documentPriorities.value.size
  }
  
  get highPriorityCount() {
    return Array.from(this.documentPriorities.value.values())
      .filter(p => p.priority >= this.cacheStrategy.value.priorityThreshold).length
  }
}

// Singleton instance
let documentPriorityInstance: DocumentPriorityService | null = null

export function useDocumentPriority() {
  if (!documentPriorityInstance) {
    documentPriorityInstance = new DocumentPriorityService()
  }
  
  return {
    calculateDocumentPriority: documentPriorityInstance.calculateDocumentPriority.bind(documentPriorityInstance),
    updateDocumentAccess: documentPriorityInstance.updateDocumentAccess.bind(documentPriorityInstance),
    getTopPriorityDocuments: documentPriorityInstance.getTopPriorityDocuments.bind(documentPriorityInstance),
    getCachedDocuments: documentPriorityInstance.getCachedDocuments.bind(documentPriorityInstance),
    shouldCacheDocument: documentPriorityInstance.shouldCacheDocument.bind(documentPriorityInstance),
    preloadSimilarDocuments: documentPriorityInstance.preloadSimilarDocuments.bind(documentPriorityInstance),
    evictLowPriorityDocuments: documentPriorityInstance.evictLowPriorityDocuments.bind(documentPriorityInstance),
    updateCacheStrategy: documentPriorityInstance.updateCacheStrategy.bind(documentPriorityInstance),
    isProcessingPriorities: computed(() => documentPriorityInstance?.isProcessingPriorities || false),
    currentCacheStrategy: computed(() => documentPriorityInstance?.currentCacheStrategy || {} as CacheStrategy),
    priorityCount: computed(() => documentPriorityInstance?.priorityCount || 0),
    highPriorityCount: computed(() => documentPriorityInstance?.highPriorityCount || 0),
  }
}