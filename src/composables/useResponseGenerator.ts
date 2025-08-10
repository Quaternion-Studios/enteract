import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { MarkdownRenderer } from './markdownRenderer'

export interface GeneratedResponse {
  text: string
  confidence: number
  contextRelevance: number
  tempoMatch: number
}

// Context-driven response generation - no predefined templates
const detectQuestionInContext = (context: string): boolean => {
  const questionIndicators = ['?', 'what ', 'how ', 'why ', 'when ', 'where ', 'who ', 'which ', 'could you', 'can you', 'would you', 'will you', 'do you']
  const lowerContext = context.toLowerCase()
  return questionIndicators.some(indicator => lowerContext.includes(indicator))
}

const detectSentiment = (context: string): 'positive' | 'negative' | 'neutral' => {
  const negativeIndicators = ['problem', 'issue', 'error', 'fail', 'wrong', 'broken', 'frustrated', 'difficult', 'hard', 'confused', 'stuck']
  const positiveIndicators = ['great', 'good', 'excellent', 'perfect', 'works', 'solved', 'happy', 'thanks', 'appreciate']
  
  const lowerContext = context.toLowerCase()
  const hasNegative = negativeIndicators.some(word => lowerContext.includes(word))
  const hasPositive = positiveIndicators.some(word => lowerContext.includes(word))
  
  if (hasNegative && !hasPositive) return 'negative'
  if (hasPositive && !hasNegative) return 'positive'
  return 'neutral'
}

export function useResponseGenerator() {
  const isGenerating = ref(false)
  const lastGeneratedResponses = ref<GeneratedResponse[]>([])

  const generateContextualResponses = async (
    context: string,
    tempo: any
  ): Promise<GeneratedResponse[]> => {
    const responses: GeneratedResponse[] = []
    const hasQuestion = detectQuestionInContext(context)
    const sentiment = detectSentiment(context)
    
    // If there's a question, prioritize answering it
    if (hasQuestion) {
      const answer = await generateDirectAnswer(context, tempo)
      if (answer) {
        responses.push(answer)
      }
    }
    
    // Generate context-appropriate responses based on sentiment and tempo
    if (sentiment === 'negative' && tempo.urgencyLevel === 'high') {
      // Quick empathetic response for urgent negative situations
      const empathyResponse = await generateEmpathyResponse(context, tempo)
      if (empathyResponse) responses.push(empathyResponse)
    }
    
    // Add a natural follow-up based on conversation flow
    const followUp = await generateNaturalFollowUp(context, tempo)
    if (followUp) responses.push(followUp)
    
    // Sort by relevance and confidence
    responses.sort((a, b) => {
      const scoreA = (a.confidence * 0.5) + (a.contextRelevance * 0.5)
      const scoreB = (b.confidence * 0.5) + (b.contextRelevance * 0.5)
      return scoreB - scoreA
    })
    
    lastGeneratedResponses.value = responses
    return responses.slice(0, 3) // Return top 3 responses
  }

  const generateDirectAnswer = async (
    context: string,
    tempo: any
  ): Promise<GeneratedResponse | null> => {
    try {
      isGenerating.value = true
      
      // Extract the most recent question from context
      const lines = context.split('\n')
      const lastQuestion = lines.reverse().find(line => detectQuestionInContext(line))
      
      if (!lastQuestion) return null
      
      // Generate a direct, concise answer
      const enhancedContext = {
        conversation: context,
        question: lastQuestion,
        instruction: 'Provide a direct, concise answer to the question. No acknowledgments or filler.',
        tempo: tempo.pace,
        urgency: tempo.urgencyLevel
      }
      
      const response = await invoke<any>('generate_typed_response', {
        context: enhancedContext
      })
      
      const responseText = response.text || response
      const formattedText = MarkdownRenderer.render(responseText)
      
      return {
        text: formattedText,
        confidence: 0.9,
        contextRelevance: 1.0,
        tempoMatch: calculateTempoMatch(responseText, tempo)
      }
    } catch (error) {
      console.error('Failed to generate direct answer:', error)
      return null
    } finally {
      isGenerating.value = false
    }
  }
  
  const generateEmpathyResponse = async (
    context: string,
    tempo: any
  ): Promise<GeneratedResponse | null> => {
    try {
      const enhancedContext = {
        conversation: context,
        instruction: 'Provide a brief, empathetic response that acknowledges the difficulty without being generic.',
        tempo: tempo.pace
      }
      
      const response = await invoke<any>('generate_typed_response', {
        context: enhancedContext
      })
      
      const responseText = response.text || response
      const formattedText = MarkdownRenderer.render(responseText)
      
      return {
        text: formattedText,
        confidence: 0.7,
        contextRelevance: 0.8,
        tempoMatch: calculateTempoMatch(responseText, tempo)
      }
    } catch (error) {
      console.error('Failed to generate empathy response:', error)
      return null
    }
  }
  
  const generateNaturalFollowUp = async (
    context: string,
    tempo: any
  ): Promise<GeneratedResponse | null> => {
    try {
      const enhancedContext = {
        conversation: context,
        instruction: 'Suggest a natural next thing to say based on the conversation flow. Be specific and contextual.',
        tempo: tempo.pace,
        urgency: tempo.urgencyLevel
      }
      
      const response = await invoke<any>('generate_typed_response', {
        context: enhancedContext
      })
      
      const responseText = response.text || response
      const formattedText = MarkdownRenderer.render(responseText)
      
      return {
        text: formattedText,
        confidence: 0.8,
        contextRelevance: 0.9,
        tempoMatch: calculateTempoMatch(responseText, tempo)
      }
    } catch (error) {
      console.error('Failed to generate follow-up:', error)
      return null
    }
  }

  const calculateTempoMatch = (responseText: string, tempo: any): number => {
    const wordCount = responseText.split(' ').length
    
    let idealLength: number
    switch (tempo.pace) {
      case 'rapid':
        idealLength = 5
        break
      case 'fast':
        idealLength = 10
        break
      case 'moderate':
        idealLength = 20
        break
      case 'slow':
        idealLength = 30
        break
      default:
        idealLength = 15
    }
    
    // Calculate how well the response length matches the tempo
    const lengthDiff = Math.abs(wordCount - idealLength)
    const matchScore = Math.max(0, 1 - (lengthDiff / idealLength))
    
    return matchScore
  }

  const generateQuickResponse = async (context: string): Promise<string> => {
    // Generate contextual quick response based on what was actually said
    const hasQuestion = detectQuestionInContext(context)
    const sentiment = detectSentiment(context)
    
    if (hasQuestion) {
      // If asked a question, give a brief direct response
      const lastLine = context.split('\n').pop() || ''
      if (lastLine.toLowerCase().includes('how')) return "Here's how..."
      if (lastLine.toLowerCase().includes('what')) return "It's..."
      if (lastLine.toLowerCase().includes('why')) return "Because..."
      if (lastLine.toLowerCase().includes('when')) return "We can schedule that..."
      return "Let me clarify that"
    }
    
    if (sentiment === 'negative') {
      return "I see the issue"
    }
    
    if (sentiment === 'positive') {
      return "Excellent"
    }
    
    // Default to acknowledging what was said naturally
    return "I understand"
  }

  const adaptResponseToTempo = (response: string, tempo: any): string => {
    if (tempo.pace === 'rapid' && response.length > 50) {
      // Shorten response for rapid pace
      const sentences = response.split('. ')
      return sentences[0] + '.'
    }
    
    if (tempo.pace === 'slow' && response.length < 30) {
      // Expand response for slow pace
      return response + " Would you like me to elaborate?"
    }
    
    return response
  }

  const scoreResponseRelevance = (response: string, context: string): number => {
    // Simple keyword matching for relevance scoring
    const contextWords = context.toLowerCase().split(/\s+/)
    const responseWords = response.toLowerCase().split(/\s+/)
    
    const commonWords = contextWords.filter(word => 
      responseWords.includes(word) && word.length > 3
    )
    
    return Math.min(1, commonWords.length / Math.max(contextWords.length * 0.1, 1))
  }

  return {
    isGenerating,
    lastGeneratedResponses,
    generateContextualResponses,
    generateDirectAnswer,
    generateQuickResponse,
    adaptResponseToTempo,
    scoreResponseRelevance
  }
}