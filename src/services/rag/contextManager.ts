import { EnhancedDocumentChunk } from './types'

export interface RagContextResult {
  context: string
  tokensUsed: number
  chunksIncluded: number
  chunksDropped: number
}

export class RagContextManager {
  // Estimated tokens per character ratio (conservative estimate)
  private static readonly CHARS_PER_TOKEN = 4

  static estimateTokens(text: string): number {
    return Math.ceil(text.length / this.CHARS_PER_TOKEN)
  }

  /**
   * Format RAG context with intelligent token management
   * Prioritizes highest-scoring chunks and gracefully truncates when needed
   */
  static formatContextForAI(
    chunks: EnhancedDocumentChunk[],
    maxRagTokens: number = 1500,
    includeScores: boolean = false
  ): RagContextResult {
    if (!chunks || chunks.length === 0) {
      return {
        context: '',
        tokensUsed: 0,
        chunksIncluded: 0,
        chunksDropped: 0
      }
    }

    // Sort chunks by relevance score (combined similarity + bm25)
    const sortedChunks = chunks.sort((a, b) => {
      const scoreA = this.calculateCombinedScore(a)
      const scoreB = this.calculateCombinedScore(b)
      return scoreB - scoreA
    })

    let context = 'Relevant document context:\n\n'
    let tokensUsed = this.estimateTokens(context)
    let chunksIncluded = 0
    let chunksDropped = 0

    // Add chunks until we hit the token limit
    for (const chunk of sortedChunks) {
      const chunkText = this.formatChunkText(chunk, includeScores)
      const chunkTokens = this.estimateTokens(chunkText)
      
      // Check if adding this chunk would exceed the limit
      if (tokensUsed + chunkTokens > maxRagTokens) {
        // Try to truncate the chunk if it's the first one and still too large
        if (chunksIncluded === 0) {
          const availableTokens = maxRagTokens - tokensUsed
          if (availableTokens > 100) { // Only truncate if we have reasonable space
            const truncatedChunk = this.truncateChunk(chunk, availableTokens, includeScores)
            context += truncatedChunk + '\n\n'
            tokensUsed = maxRagTokens // Use all available tokens
            chunksIncluded = 1
            chunksDropped = sortedChunks.length - 1
            break
          }
        }
        // Otherwise, we can't fit any more chunks
        chunksDropped = sortedChunks.length - chunksIncluded
        break
      }

      context += chunkText + '\n\n'
      tokensUsed += chunkTokens
      chunksIncluded++
    }

    return {
      context: context.trim(),
      tokensUsed,
      chunksIncluded,
      chunksDropped
    }
  }

  /**
   * Calculate combined relevance score for a chunk
   */
  private static calculateCombinedScore(chunk: EnhancedDocumentChunk): number {
    const similarityScore = chunk.similarity_score || 0
    const bm25Score = chunk.bm25_score || 0
    
    // Weighted combination: 70% semantic similarity, 30% BM25
    return (similarityScore * 0.7) + (bm25Score * 0.3)
  }

  /**
   * Format a chunk for inclusion in context
   */
  private static formatChunkText(chunk: EnhancedDocumentChunk, includeScores: boolean): string {
    let text = chunk.content.trim()
    
    if (includeScores) {
      const combinedScore = this.calculateCombinedScore(chunk)
      text = `[Score: ${combinedScore.toFixed(3)}] ${text}`
    }
    
    return text
  }

  /**
   * Intelligently truncate a chunk to fit within token limits
   */
  private static truncateChunk(
    chunk: EnhancedDocumentChunk,
    availableTokens: number,
    includeScores: boolean
  ): string {
    const availableChars = availableTokens * this.CHARS_PER_TOKEN
    let content = chunk.content.trim()
    
    if (content.length <= availableChars) {
      return this.formatChunkText(chunk, includeScores)
    }

    // Try to truncate at sentence boundaries
    const sentences = content.split(/[.!?]+/)
    let truncatedContent = ''
    
    for (const sentence of sentences) {
      const candidateContent = truncatedContent + sentence + '.'
      if (candidateContent.length > availableChars - 20) { // Leave some buffer
        break
      }
      truncatedContent = candidateContent
    }

    // If no good sentence boundary found, truncate at word boundary
    if (truncatedContent.length < availableChars * 0.5) {
      const words = content.split(' ')
      truncatedContent = ''
      
      for (const word of words) {
        const candidateContent = truncatedContent + ' ' + word
        if (candidateContent.length > availableChars - 20) {
          break
        }
        truncatedContent = candidateContent
      }
    }

    // Fallback: hard truncate
    if (truncatedContent.length === 0) {
      truncatedContent = content.substring(0, availableChars - 20)
    }

    const truncatedChunk = { ...chunk, content: truncatedContent + '...' }
    return this.formatChunkText(truncatedChunk, includeScores)
  }

  /**
   * Determine optimal token allocation between RAG context and conversation history
   */
  static calculateOptimalTokenAllocation(
    ragChunks: EnhancedDocumentChunk[],
    totalTokenBudget: number = 4000,
    minConversationTokens: number = 1500,
    maxRagTokens: number = 2000
  ): { ragTokens: number; conversationTokens: number } {
    if (!ragChunks || ragChunks.length === 0) {
      return {
        ragTokens: 0,
        conversationTokens: totalTokenBudget
      }
    }

    // Estimate total RAG content size
    const totalRagTokens = ragChunks.reduce((sum, chunk) => {
      return sum + this.estimateTokens(chunk.content)
    }, 0)

    // Calculate optimal allocation
    let allocatedRagTokens = Math.min(totalRagTokens, maxRagTokens)
    let allocatedConversationTokens = totalTokenBudget - allocatedRagTokens

    // Ensure minimum conversation tokens
    if (allocatedConversationTokens < minConversationTokens) {
      allocatedConversationTokens = minConversationTokens
      allocatedRagTokens = totalTokenBudget - minConversationTokens
      
      // Ensure we don't go negative
      if (allocatedRagTokens < 0) {
        allocatedRagTokens = 0
        allocatedConversationTokens = totalTokenBudget
      }
    }

    return {
      ragTokens: allocatedRagTokens,
      conversationTokens: allocatedConversationTokens
    }
  }

  /**
   * Validate that the context fits within limits and provide suggestions
   */
  static validateContextSize(
    ragContext: string,
    conversationContext: string,
    totalLimit: number = 4000
  ): { isValid: boolean; ragTokens: number; conversationTokens: number; suggestions?: string[] } {
    const ragTokens = this.estimateTokens(ragContext)
    const conversationTokens = this.estimateTokens(conversationContext)
    const totalTokens = ragTokens + conversationTokens

    const isValid = totalTokens <= totalLimit
    const suggestions: string[] = []

    if (!isValid) {
      const excess = totalTokens - totalLimit
      
      if (ragTokens > 1500) {
        suggestions.push(`Consider reducing RAG context by ~${Math.ceil(excess * 0.7)} tokens`)
      }
      
      if (conversationTokens > 2000) {
        suggestions.push(`Consider reducing conversation history by ~${Math.ceil(excess * 0.3)} tokens`)
      }
      
      suggestions.push(`Total excess: ${excess} tokens`)
    }

    return {
      isValid,
      ragTokens,
      conversationTokens,
      suggestions: suggestions.length > 0 ? suggestions : undefined
    }
  }
}