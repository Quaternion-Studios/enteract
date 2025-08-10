import { ref } from 'vue'
import { MarkdownRenderer } from './markdownRenderer'
// Vite raw import for markdown templates (no Tauri fs needed)
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore - Vite will inline this as a string
import templatesRaw from '../resources/live-ai-templates.md?raw'
import { invoke } from '@tauri-apps/api/core'

export interface ResponseTemplate {
  type: string
  templates: string[]
  contextRequired: boolean
  minConfidence: number
}

export interface GeneratedResponse {
  text: string
  type: string
  confidence: number
  contextRelevance: number
  tempoMatch: number
}

const responseTemplates: ResponseTemplate[] = [
  {
    type: 'quick-acknowledgment',
    templates: [
      "I understand",
      "Got it",
      "That makes sense",
      "I see what you mean",
      "Absolutely"
    ],
    contextRequired: false,
    minConfidence: 0.3
  },
  {
    type: 'clarification',
    templates: [
      "Could you elaborate on {topic}?",
      "What do you mean by {point}?",
      "Can you explain {concept} further?",
      "I'd like to understand more about {subject}"
    ],
    contextRequired: true,
    minConfidence: 0.5
  },
  {
    type: 'technical-insight',
    templates: [
      "Based on the {technology}, we should consider {approach}",
      "The {component} could be optimized by {method}",
      "Have you considered using {solution} for this?",
      "This reminds me of {pattern} pattern"
    ],
    contextRequired: true,
    minConfidence: 0.7
  },
  {
    type: 'empathy',
    templates: [
      "I understand that can be frustrating",
      "That sounds challenging",
      "I appreciate your patience with this",
      "Let's work through this together"
    ],
    contextRequired: false,
    minConfidence: 0.4
  },
  {
    type: 'solution',
    templates: [
      "Here's what we can try: {steps}",
      "One approach would be to {action}",
      "Let me suggest {solution}",
      "We could resolve this by {method}"
    ],
    contextRequired: true,
    minConfidence: 0.6
  },
  {
    type: 'follow-up',
    templates: [
      "How did that work out?",
      "Did that solve the issue?",
      "Is there anything else about {topic} you'd like to discuss?",
      "What are your thoughts on {subject}?"
    ],
    contextRequired: true,
    minConfidence: 0.5
  },
  {
    type: 'engagement-question',
    templates: [
      "What's your experience with {topic}?",
      "How do you typically handle {situation}?",
      "What are your priorities for {subject}?",
      "What would be your ideal outcome?"
    ],
    contextRequired: true,
    minConfidence: 0.5
  },
  {
    type: 'summary',
    templates: [
      "So to summarize: {points}",
      "The key points are: {summary}",
      "Let me make sure I understand: {recap}",
      "In essence, {conclusion}"
    ],
    contextRequired: true,
    minConfidence: 0.6
  }
]

export function useResponseGenerator() {
  const isGenerating = ref(false)
  const lastGeneratedResponses = ref<GeneratedResponse[]>([])
  const templateMarkdown = ref<string>('')
  const parsedBuckets = ref<Record<string, string[]>>({})

  // Lazy load markdown templates from resources
  const ensureTemplatesLoaded = async () => {
    if (templateMarkdown.value) return
    try {
      templateMarkdown.value = (templatesRaw || '').trim()
      parsedBuckets.value = parseTemplateMarkdown(templateMarkdown.value)
    } catch (e) {
      console.warn('Live AI templates not found, falling back to defaults', e)
      templateMarkdown.value = ''
      parsedBuckets.value = {}
    }
  }

  const parseTemplateMarkdown = (md: string): Record<string, string[]> => {
    const buckets: Record<string, string[]> = {}
    const sections = md.split(/\n\n+/)
    let currentType: string | null = null
    for (const block of sections) {
      const trimmed = block.trim()
      if (!trimmed) continue
      if (/^(engagement-question|contextual)$/i.test(trimmed)) {
        currentType = trimmed.toLowerCase()
        if (!buckets[currentType]) buckets[currentType] = []
        continue
      }
      // If no currentType yet, infer from keywords
      if (!currentType) {
        currentType = trimmed.toLowerCase().includes('acknowledge:') ? 'contextual' : 'engagement-question'
        if (!buckets[currentType]) buckets[currentType] = []
      }
      buckets[currentType].push(trimmed)
    }
    return buckets
  }

  const generateMultipleResponseTypes = async (
    context: string,
    responseTypes: string[],
    tempo: any
  ): Promise<GeneratedResponse[]> => {
    const responses: GeneratedResponse[] = []
    
    for (const type of responseTypes) {
      const response = await generateResponseByType(context, type, tempo)
      if (response) {
        responses.push(response)
      }
    }
    
    // Sort by confidence and tempo match
    responses.sort((a, b) => {
      const scoreA = (a.confidence * 0.4) + (a.tempoMatch * 0.3) + (a.contextRelevance * 0.3)
      const scoreB = (b.confidence * 0.4) + (b.tempoMatch * 0.3) + (b.contextRelevance * 0.3)
      return scoreB - scoreA
    })
    
    lastGeneratedResponses.value = responses
    return responses.slice(0, 3) // Return top 3 responses
  }

  const generateResponseByType = async (
    context: string,
    type: string,
    tempo: any
  ): Promise<GeneratedResponse | null> => {
    await ensureTemplatesLoaded()
    const template = responseTemplates.find(t => t.type === type)
    if (!template) return null
    
    try {
      isGenerating.value = true
      
      // For quick responses with low context requirement, use templates
      if (!template.contextRequired && tempo.pace === 'rapid') {
        const randomTemplate = template.templates[Math.floor(Math.random() * template.templates.length)]
        return {
          text: randomTemplate,
          type,
          confidence: 0.7,
          contextRelevance: 0.5,
          tempoMatch: 1.0
        }
      }
      
      // Use curated markdown snippets if available for this type first
      const bucketKey = (type === 'quick-acknowledgment') ? 'contextual' : (type.includes('engagement') ? 'engagement-question' : 'contextual')
      const candidates = parsedBuckets.value[bucketKey] || []
      if (candidates.length > 0) {
        const pick = candidates[Math.floor(Math.random() * candidates.length)]
        const rendered = MarkdownRenderer.render(pick)
        return {
          text: stripHtml(rendered),
          type,
          confidence: 0.8,
          contextRelevance: 0.6,
          tempoMatch: calculateTempoMatch(pick, tempo)
        }
      }

      // For context-aware responses, use AI generation when markdown has no suitable entry
      const enhancedContext = {
        conversation: context,
        responseType: type,
        tempo: tempo.pace,
        urgency: tempo.urgencyLevel,
        conversationType: tempo.conversationType,
        templates: template.templates
      }
      
      const response = await invoke<GeneratedResponse>('generate_typed_response', {
        context: enhancedContext
      })
      
      // Calculate tempo match score
      const tempoMatch = calculateTempoMatch(response.text, tempo)
      
      return {
        ...response,
        type,
        tempoMatch
      }
    } catch (error) {
      console.error(`Failed to generate ${type} response:`, error)
      return null
    } finally {
      isGenerating.value = false
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
    // Generate a very quick, contextual response for rapid conversations
    const quickTemplates = [
      "Interesting point",
      "That's helpful",
      "Good idea",
      "Let me think about that",
      "Makes sense"
    ]
    
    if (context.includes('?')) {
      return "Let me check on that"
    }
    
    return quickTemplates[Math.floor(Math.random() * quickTemplates.length)]
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
    generateMultipleResponseTypes,
    generateResponseByType,
    generateQuickResponse,
    adaptResponseToTempo,
    scoreResponseRelevance,
    responseTemplates
  }
}

// Helper to strip HTML tags from rendered markdown where we only need text
function stripHtml(html: string): string {
  const tmp = document.createElement('div')
  tmp.innerHTML = html
  return tmp.textContent || tmp.innerText || ''
}