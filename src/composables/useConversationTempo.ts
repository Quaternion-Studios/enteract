import { ref, computed } from 'vue'

export interface ConversationTempo {
  pace: 'slow' | 'moderate' | 'fast' | 'rapid'
  averageMessageInterval: number
  lastSpeaker: 'user' | 'system' | null
  turnTakingPattern: 'alternating' | 'one-sided' | 'balanced'
  urgencyLevel: 'low' | 'medium' | 'high'
  conversationType: 'casual' | 'business' | 'technical' | 'support'
}

export interface TempoMetrics {
  messageTimestamps: number[]
  speakerTransitions: number
  averageResponseTime: number
  peakActivityTime: number
  currentActivityLevel: number
}

export function useConversationTempo() {
  const currentTempo = ref<ConversationTempo>({
    pace: 'moderate',
    averageMessageInterval: 5000,
    lastSpeaker: null,
    turnTakingPattern: 'balanced',
    urgencyLevel: 'medium',
    conversationType: 'casual'
  })

  const tempoMetrics = ref<TempoMetrics>({
    messageTimestamps: [],
    speakerTransitions: 0,
    averageResponseTime: 3000,
    peakActivityTime: 0,
    currentActivityLevel: 0
  })

  const dynamicDebounceTime = computed(() => {
    switch (currentTempo.value.pace) {
      case 'rapid':
        return 500
      case 'fast':
        return 1000
      case 'moderate':
        return 2000
      case 'slow':
        return 3000
      default:
        return 2000
    }
  })

  const dynamicAnalysisInterval = computed(() => {
    switch (currentTempo.value.pace) {
      case 'rapid':
        return 1000
      case 'fast':
        return 2000
      case 'moderate':
        return 3500
      case 'slow':
        return 5000
      default:
        return 3500
    }
  })

  const shouldTriggerPreemptiveAnalysis = computed(() => {
    return currentTempo.value.pace === 'rapid' || 
           currentTempo.value.pace === 'fast' ||
           currentTempo.value.urgencyLevel === 'high'
  })

  const suggestedResponseTypes = computed(() => {
    const types = []
    
    if (currentTempo.value.urgencyLevel === 'high') {
      types.push('quick-acknowledgment', 'clarification')
    }
    
    if (currentTempo.value.conversationType === 'technical') {
      types.push('technical-insight', 'code-suggestion')
    }
    
    if (currentTempo.value.conversationType === 'support') {
      types.push('empathy', 'solution', 'follow-up')
    }
    
    if (currentTempo.value.turnTakingPattern === 'one-sided') {
      types.push('engagement-question', 'summary')
    }
    
    if (types.length === 0) {
      types.push('contextual', 'follow-up', 'clarification')
    }
    
    return types
  })

  const analyzeConversationTempo = (messages: any[]) => {
    if (messages.length < 2) return currentTempo.value

    const recentMessages = messages.slice(-20)
    const timestamps = recentMessages.map(m => m.timestamp)
    
    const intervals: number[] = []
    for (let i = 1; i < timestamps.length; i++) {
      intervals.push(timestamps[i] - timestamps[i - 1])
    }

    const avgInterval = intervals.length > 0 
      ? intervals.reduce((a, b) => a + b, 0) / intervals.length 
      : 5000

    let pace: ConversationTempo['pace'] = 'moderate'
    if (avgInterval < 1000) pace = 'rapid'
    else if (avgInterval < 2500) pace = 'fast'
    else if (avgInterval < 5000) pace = 'moderate'
    else pace = 'slow'

    let speakerTransitions = 0
    let lastSource = recentMessages[0]?.source
    for (let i = 1; i < recentMessages.length; i++) {
      if (recentMessages[i].source !== lastSource) {
        speakerTransitions++
        lastSource = recentMessages[i].source
      }
    }

    const transitionRate = speakerTransitions / recentMessages.length
    let turnTakingPattern: ConversationTempo['turnTakingPattern'] = 'balanced'
    if (transitionRate < 0.2) turnTakingPattern = 'one-sided'
    else if (transitionRate > 0.7) turnTakingPattern = 'alternating'

    const recentIntervals = intervals.slice(-5)
    const recentAvg = recentIntervals.length > 0
      ? recentIntervals.reduce((a, b) => a + b, 0) / recentIntervals.length
      : avgInterval
    
    let urgencyLevel: ConversationTempo['urgencyLevel'] = 'medium'
    if (recentAvg < avgInterval * 0.5 && pace === 'rapid') urgencyLevel = 'high'
    else if (recentAvg > avgInterval * 1.5 && pace === 'slow') urgencyLevel = 'low'

    const lastMessage = recentMessages[recentMessages.length - 1]
    const lastSpeaker = lastMessage?.source === 'microphone' ? 'user' : 'system'

    const conversationType = detectConversationType(recentMessages)

    const newTempo: ConversationTempo = {
      pace,
      averageMessageInterval: avgInterval,
      lastSpeaker,
      turnTakingPattern,
      urgencyLevel,
      conversationType
    }

    currentTempo.value = newTempo

    tempoMetrics.value = {
      messageTimestamps: timestamps,
      speakerTransitions,
      averageResponseTime: avgInterval,
      peakActivityTime: Math.min(...intervals),
      currentActivityLevel: recentMessages.length / 20
    }

    return newTempo
  }

  const detectConversationType = (messages: any[]): ConversationTempo['conversationType'] => {
    const recentContent = messages.slice(-10).map(m => m.content.toLowerCase()).join(' ')
    
    const technicalKeywords = ['code', 'function', 'error', 'debug', 'api', 'database', 'server', 'bug', 'deploy']
    const businessKeywords = ['meeting', 'schedule', 'deadline', 'project', 'budget', 'client', 'proposal']
    const supportKeywords = ['help', 'issue', 'problem', 'fix', 'not working', 'broken', 'assist']
    
    const technicalScore = technicalKeywords.filter(k => recentContent.includes(k)).length
    const businessScore = businessKeywords.filter(k => recentContent.includes(k)).length
    const supportScore = supportKeywords.filter(k => recentContent.includes(k)).length
    
    if (technicalScore >= 3) return 'technical'
    if (businessScore >= 3) return 'business'
    if (supportScore >= 2) return 'support'
    
    return 'casual'
  }

  const getResponsePriority = (): 'immediate' | 'soon' | 'normal' | 'low' => {
    if (currentTempo.value.urgencyLevel === 'high' && currentTempo.value.pace === 'rapid') {
      return 'immediate'
    }
    if (currentTempo.value.urgencyLevel === 'high' || currentTempo.value.pace === 'fast') {
      return 'soon'
    }
    if (currentTempo.value.urgencyLevel === 'low' && currentTempo.value.pace === 'slow') {
      return 'low'
    }
    return 'normal'
  }

  const shouldWaitForUserToFinish = (): boolean => {
    if (currentTempo.value.lastSpeaker !== 'user') return false
    
    const timeSinceLastMessage = Date.now() - (tempoMetrics.value.messageTimestamps.slice(-1)[0] || 0)
    const expectedPauseTime = currentTempo.value.averageMessageInterval * 0.8
    
    return timeSinceLastMessage < expectedPauseTime
  }

  return {
    currentTempo,
    tempoMetrics,
    dynamicDebounceTime,
    dynamicAnalysisInterval,
    shouldTriggerPreemptiveAnalysis,
    suggestedResponseTypes,
    analyzeConversationTempo,
    detectConversationType,
    getResponsePriority,
    shouldWaitForUserToFinish
  }
}