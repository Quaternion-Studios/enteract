<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { 
  XMarkIcon, 
  PlayIcon, 
  StopIcon,
  ClipboardDocumentIcon,
  CheckIcon,
  BoltIcon,
  ChatBubbleLeftRightIcon,
  QuestionMarkCircleIcon,
  HandThumbUpIcon,
  ArrowPathIcon,
  PhoneIcon,
  UserGroupIcon,
  ComputerDesktopIcon,
  SparklesIcon,
  CogIcon,
  DocumentTextIcon
} from '@heroicons/vue/24/outline'

interface SuggestionItem {
  id: string
  text: string
  timestamp: number
  contextLength: number
  responseType?: string
  priority?: 'immediate' | 'soon' | 'normal' | 'low'
  confidence?: number
}

interface ConversationTempo {
  pace: 'slow' | 'moderate' | 'fast' | 'rapid'
  averageMessageInterval: number
  lastSpeaker: 'user' | 'system' | null
  turnTakingPattern: 'alternating' | 'one-sided' | 'balanced'
  urgencyLevel: 'low' | 'medium' | 'high'
  conversationType: 'casual' | 'business' | 'technical' | 'support'
}

 interface Props {
  show: boolean
  isActive?: boolean
  processing?: boolean
  response?: string
  suggestions?: SuggestionItem[]
  error?: string | null
  sessionId?: string | null
  conversationTempo?: ConversationTempo | null
  // AI Assistant props (simplified)
  aiProcessing?: boolean
  aiResponse?: string
  aiError?: string | null
  messageCount?: number
  // When true, expand to overlay fullscreen
  fullScreen?: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'toggle-live'): void
  (e: 'ai-query', query: string): void
  (e: 'update-system-prompt', prompt: string): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// UI State
const copiedStates = ref<Record<string, boolean>>({})
const showSettings = ref(false)
const customPrompt = ref('')
const selectedTemplate = ref('sales')
const promptValidationError = ref<string | null>(null)

// Predefined prompt templates for different scenarios
const promptTemplates = ref([
  {
    id: 'sales',
    name: 'Sales Call',
    icon: '💼',
    description: 'For sales conversations and client meetings',
    prompt: `You are a sales conversation assistant. Help with:
- Acknowledge customer concerns professionally
- Ask clarifying questions about needs and budget  
- Suggest next steps to move the sale forward
- Handle objections with empathy

Keep responses under 15 words. Focus on building rapport and closing deals.
Format: **Acknowledge:** [response] **Question:** [response] **Action:** [response]`
  },
  {
    id: 'support',
    name: 'Customer Support',
    icon: '🤝',
    description: 'For customer service and troubleshooting',
    prompt: `You are a customer support assistant. Help with:
- Show empathy for customer frustrations
- Ask specific questions to diagnose issues
- Provide clear next steps to resolve problems
- Follow up to ensure satisfaction

Keep responses under 15 words. Focus on problem-solving and customer satisfaction.
Format: **Acknowledge:** [response] **Question:** [response] **Action:** [response]`
  },
  {
    id: 'technical',
    name: 'Technical Meeting',
    icon: '⚡',
    description: 'For technical discussions and problem-solving',
    prompt: `You are a technical conversation assistant. Help with:
- Acknowledge technical points accurately
- Ask for clarification on specifications
- Suggest implementation approaches
- Identify potential issues or blockers

Keep responses under 15 words. Focus on technical accuracy and problem-solving.
Format: **Acknowledge:** [response] **Question:** [response] **Action:** [response]`
  },
  {
    id: 'general',
    name: 'General Business',
    icon: '💬',
    description: 'For general business conversations',
    prompt: `You are a business conversation assistant. Help with:
- Acknowledge points professionally
- Ask relevant follow-up questions
- Suggest appropriate next steps
- Keep conversation productive

Keep responses under 15 words. Focus on professionalism and productivity.
Format: **Acknowledge:** [response] **Question:** [response] **Action:** [response]`
  },
  {
    id: 'custom',
    name: 'Custom',
    icon: '✏️',
    description: 'Create your own custom prompt',
    prompt: ''
  }
])

// 4 essential quick actions for live calls
const quickActions = ref([
  { 
    icon: HandThumbUpIcon, 
    text: "I understand", 
    query: "What's a good acknowledgment response?",
    color: "green"
  },
  { 
    icon: QuestionMarkCircleIcon, 
    text: "Good question", 
    query: "What clarifying question should I ask?",
    color: "blue"
  },
  { 
    icon: ArrowPathIcon, 
    text: "Let me check", 
    query: "What should I do next?",
    color: "purple"
  },
  { 
    icon: ChatBubbleLeftRightIcon, 
    text: "Tell me more", 
    query: "How can I continue this conversation?",
    color: "orange"
  }
])

const handleClose = () => emit('close')
const handleToggleLive = () => emit('toggle-live')

const handleQuickAction = (action: any) => {
  if (props.isActive) {
    // Copy the quick text directly
    copyToClipboard(action.text, action.text)
  } else {
    // If not active, send AI query
    emit('ai-query', action.query)
  }
}

const copyToClipboard = async (text: string, id?: string) => {
  if (!text) return
  
  try {
    await navigator.clipboard.writeText(text)
    
    if (id) {
      copiedStates.value[id] = true
      setTimeout(() => {
        copiedStates.value[id] = false
      }, 1500)
    }
  } catch (error) {
    console.error('Failed to copy:', error)
  }
}

const getConversationIcon = computed(() => {
  switch (props.conversationTempo?.conversationType) {
    case 'business': return PhoneIcon
    case 'support': return UserGroupIcon
    case 'technical': return ComputerDesktopIcon
    default: return ChatBubbleLeftRightIcon
  }
})

const statusText = computed(() => {
  if (!props.isActive) return 'Start Live Assistant'
  if (props.processing) return 'Listening...'
  
  const tempo = props.conversationTempo
  if (tempo) {
    const typeMap = {
      business: '💼',
      support: '🤝',
      technical: '⚡',
      casual: '💬'
    }
    return `${typeMap[tempo.conversationType] || '💬'} Live`
  }
  
  return 'Live Active'
})

const currentTemplate = computed(() => {
  return promptTemplates.value.find(t => t.id === selectedTemplate.value)
})

// Removed unused effectivePrompt computed

// Initialize with sales template
const initializePrompt = () => {
  const template = promptTemplates.value.find(t => t.id === selectedTemplate.value)
  if (template && template.prompt) {
    emit('update-system-prompt', template.prompt)
  }
}

const applyTemplate = (templateId: string) => {
  selectedTemplate.value = templateId
  const template = promptTemplates.value.find(t => t.id === templateId)
  
  if (template) {
    if (templateId === 'custom') {
      // For custom, validate before emitting
      const validation = validatePrompt(customPrompt.value)
      promptValidationError.value = validation.error
      
      if (validation.isValid) {
        emit('update-system-prompt', customPrompt.value)
      }
    } else {
      // Clear any validation errors for pre-defined templates
      promptValidationError.value = null
      emit('update-system-prompt', template.prompt)
    }
  }
}

const updateCustomPrompt = () => {
  if (selectedTemplate.value === 'custom') {
    const validation = validatePrompt(customPrompt.value)
    promptValidationError.value = validation.error
    
    // Save to localStorage even if invalid (for persistence)
    saveCustomPrompt()
    
    if (validation.isValid) {
      emit('update-system-prompt', customPrompt.value)
    }
  }
}

const validatePrompt = (prompt: string) => {
  const trimmed = prompt.trim()
  const isValid = trimmed.length >= 10 && trimmed.length <= 2000
  return {
    isValid,
    error: !isValid ? 
      (trimmed.length < 10 ? 'Prompt must be at least 10 characters' : 
       'Prompt must be less than 2000 characters') : null
  }
}

// Load saved custom prompt from localStorage
const loadSavedPrompt = () => {
  try {
    const saved = localStorage.getItem('liveai-custom-prompt')
    if (saved) {
      customPrompt.value = saved
    }
  } catch (error) {
    console.warn('Failed to load saved custom prompt:', error)
  }
}

// Save custom prompt to localStorage
const saveCustomPrompt = () => {
  try {
    localStorage.setItem('liveai-custom-prompt', customPrompt.value)
  } catch (error) {
    console.warn('Failed to save custom prompt:', error)
  }
}

// Initialize on mount
watch(() => props.show, (newValue) => {
  if (newValue) {
    loadSavedPrompt()
    initializePrompt()
  }
}, { immediate: true })
</script>

<template>
  <div v-if="show" :class="['live-assistant', 'live-ai-drawer', { 'fullscreen': props.fullScreen || props.isActive }]">
    <!-- Minimal Header -->
    <div class="assistant-header">
      <div class="header-left">
        <component :is="getConversationIcon" class="w-4 h-4 text-blue-500" />
        <span class="header-title">{{ statusText }}</span>
        <div v-if="isActive" class="live-dot"></div>
      </div>
      <div class="header-right">
        <button 
          @click="showSettings = !showSettings" 
          class="settings-button"
          :class="{ 'active': showSettings }"
          title="Settings"
        >
          <CogIcon class="w-4 h-4" />
        </button>
        <button @click="handleClose" class="close-button">
          <XMarkIcon class="w-4 h-4" />
        </button>
      </div>
    </div>
    
    <!-- Settings Panel -->
    <div v-if="showSettings" class="settings-panel">
      <div class="settings-header">
        <DocumentTextIcon class="w-4 h-4 text-purple-400" />
        <span class="text-sm font-medium text-white/90">Conversation Type</span>
      </div>
      
      <!-- Template Selection -->
      <div class="template-grid">
        <button
          v-for="template in promptTemplates"
          :key="template.id"
          @click="applyTemplate(template.id)"
          class="template-btn"
          :class="{ 'selected': selectedTemplate === template.id }"
        >
          <span class="template-icon">{{ template.icon }}</span>
          <div class="template-info">
            <span class="template-name">{{ template.name }}</span>
            <span class="template-desc">{{ template.description }}</span>
          </div>
        </button>
      </div>
      
      <!-- Custom Prompt Editor -->
      <div v-if="selectedTemplate === 'custom'" class="custom-prompt">
        <label class="prompt-label">Custom System Prompt:</label>
        <textarea
          v-model="customPrompt"
          @input="updateCustomPrompt"
          class="prompt-textarea"
          :class="{ 'error': promptValidationError }"
          rows="4"
          placeholder="Enter your custom prompt here..."
        />
        <div v-if="promptValidationError" class="validation-error">
          {{ promptValidationError }}
        </div>
        <div class="prompt-counter">
          {{ customPrompt.length }}/2000
        </div>
      </div>
      
      <!-- Current Template Preview -->
      <div v-else-if="currentTemplate" class="template-preview">
        <div class="preview-header">
          <span class="text-xs text-white/60">Current Template:</span>
          <span class="text-xs text-blue-400">{{ currentTemplate.name }}</span>
        </div>
        <div class="preview-content">
          <p class="text-xs text-white/70 leading-relaxed">{{ currentTemplate.prompt.split('\n')[0] }}</p>
        </div>
      </div>
    </div>
    
    <!-- Main Content -->
    <div class="assistant-content">
      <!-- Start/Stop Toggle -->
      <div class="toggle-section">
        <button 
          @click="handleToggleLive" 
          class="toggle-button"
          :class="{ 'active': isActive }"
        >
          <component :is="isActive ? StopIcon : PlayIcon" class="w-4 h-4" />
          <span>{{ isActive ? 'Stop' : 'Start' }}</span>
        </button>
      </div>
      
      <!-- Quick Actions (4 buttons) -->
      <div class="quick-actions-grid">
        <button
          v-for="action in quickActions"
          :key="action.text"
          @click="handleQuickAction(action)"
          class="quick-action"
          :class="[
            `action-${action.color}`,
            { 'copied': copiedStates[action.text] }
          ]"
        >
          <component 
            :is="copiedStates[action.text] ? CheckIcon : action.icon" 
            class="w-5 h-5" 
          />
          <span class="action-text">{{ action.text }}</span>
        </button>
      </div>
      
      <!-- Large Recommendations Area -->
      <div class="recommendations-area">
        <div v-if="processing || aiProcessing" class="processing">
          <BoltIcon class="w-5 h-5 animate-pulse text-blue-500" />
          <span>Thinking...</span>
        </div>
        
        <div v-else-if="suggestions && suggestions.length > 0" class="suggestions">
          <div class="suggestions-header">
            <span class="suggestions-title">AI Suggestions</span>
            <span class="suggestions-count">{{ suggestions.length }}</span>
          </div>
          <!-- Top 2 big, readable recommendations -->
          <div class="top-recommendations" v-if="suggestions.slice(0, 2).length">
            <div
              v-for="suggestion in suggestions.slice(0, 2)"
              :key="suggestion.id + '-top'"
              @click="copyToClipboard(suggestion.text, suggestion.id)"
              class="top-recommendation"
              :class="{ 'urgent': suggestion.priority === 'immediate', 'copied': copiedStates[suggestion.id] }"
            >
              <p class="top-recommendation-text">{{ suggestion.text }}</p>
              <div class="top-recommendation-meta">
                <span class="suggestion-type">{{ suggestion.responseType || 'response' }}</span>
                <ClipboardDocumentIcon v-if="!copiedStates[suggestion.id]" class="w-3 h-3" />
                <CheckIcon v-else class="w-3 h-3 text-green-500" />
              </div>
            </div>
          </div>
          <div class="suggestions-list">
            <div
              v-for="suggestion in suggestions.slice(2, 6)"
              :key="suggestion.id"
              @click="copyToClipboard(suggestion.text, suggestion.id)"
              class="suggestion"
              :class="{ 
                'urgent': suggestion.priority === 'immediate',
                'copied': copiedStates[suggestion.id]
              }"
            >
              <div class="suggestion-content">
                <p class="suggestion-text">{{ suggestion.text }}</p>
                <div class="suggestion-meta">
                  <span class="suggestion-type">{{ suggestion.responseType || 'response' }}</span>
                  <ClipboardDocumentIcon v-if="!copiedStates[suggestion.id]" class="w-3 h-3" />
                  <CheckIcon v-else class="w-3 h-3 text-green-500" />
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div v-else-if="aiResponse" class="ai-response">
          <div class="response-header">
            <SparklesIcon class="w-4 h-4 text-blue-500" />
            <span class="response-title">AI Analysis</span>
            <button 
              @click="copyToClipboard(aiResponse, 'ai-response')"
              class="copy-btn"
              :class="{ 'copied': copiedStates['ai-response'] }"
            >
              <component 
                :is="copiedStates['ai-response'] ? CheckIcon : ClipboardDocumentIcon" 
                class="w-3 h-3" 
              />
            </button>
          </div>
          <p class="response-text">{{ aiResponse }}</p>
        </div>
        
        <div v-else class="empty-state">
          <div class="empty-icon">
            <component :is="isActive ? BoltIcon : SparklesIcon" 
              class="w-8 h-8 text-gray-400" 
            />
          </div>
          <p class="empty-title">
            {{ isActive ? 'Listening for context...' : 'Ready to assist' }}
          </p>
          <p class="empty-subtitle">
            {{ isActive ? 'AI suggestions will appear here' : 'Choose conversation type in settings' }}
          </p>
        </div>
      </div>
      
      <!-- Error Display -->
      <div v-if="error || aiError" class="error-display">
        <XMarkIcon class="w-4 h-4 text-red-400" />
        <span>{{ error || aiError }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.live-assistant {
  @apply bg-white/[0.02] backdrop-blur-xl border-l border-white/10;
  @apply flex flex-col h-full;
  width: 340px;
  min-width: 340px;
  max-width: 400px;
}

.live-assistant.fullscreen {
  position: absolute;
  inset: 0;
  width: 100%;
  min-width: 0;
  max-width: none;
  border-left-width: 0;
  z-index: 40;
  @apply border border-white/10 rounded-none;
}

.assistant-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-white/10;
  @apply bg-gradient-to-r from-blue-500/5 to-transparent;
}

.header-left {
  @apply flex items-center gap-2;
}

.header-right {
  @apply flex items-center gap-1;
}

.header-title {
  @apply text-sm font-medium text-white/90;
}

.live-dot {
  @apply w-2 h-2 bg-green-400 rounded-full animate-pulse;
}

.settings-button {
  @apply p-1.5 rounded-lg hover:bg-white/10 transition-colors;
  @apply text-white/40 hover:text-white/70;
}

.settings-button.active {
  @apply bg-purple-500/20 text-purple-400;
}

.close-button {
  @apply p-1.5 rounded-lg hover:bg-white/10 transition-colors;
  @apply text-white/60 hover:text-white/90;
}

.settings-panel {
  @apply border-b border-white/10 bg-white/[0.02] p-3;
}

.settings-header {
  @apply flex items-center gap-2 mb-3;
}

.template-grid {
  @apply grid grid-cols-2 gap-2 mb-3;
}

.template-btn {
  @apply flex items-center gap-2 p-2 rounded-lg;
  @apply bg-white/[0.02] hover:bg-white/[0.05] border border-white/10;
  @apply transition-all duration-200 text-left;
}

.template-btn.selected {
  @apply bg-blue-500/20 border-blue-500/40;
}

.template-icon {
  @apply text-lg flex-shrink-0;
}

.template-info {
  @apply flex flex-col min-w-0;
}

.template-name {
  @apply text-xs font-medium text-white/90;
}

.template-desc {
  @apply text-xs text-white/50 leading-tight;
  font-size: 10px;
}

.custom-prompt {
  @apply space-y-2;
}

.prompt-label {
  @apply text-xs text-white/70 font-medium;
}

.prompt-textarea {
  @apply w-full border border-white/20 rounded-lg px-2 py-2 text-white;
  @apply placeholder-white/50 focus:outline-none focus:border-purple-500/50;
  @apply bg-white/[0.03] transition-all duration-200 resize-none;
  font-size: 11px;
}

.prompt-textarea.error {
  @apply border-red-500/50 focus:border-red-500/70;
}

.validation-error {
  @apply text-xs text-red-400 mt-1;
}

.prompt-counter {
  @apply text-xs text-white/40 mt-1 text-right;
}

.template-preview {
  @apply space-y-2;
}

.preview-header {
  @apply flex items-center justify-between;
}

.preview-content {
  @apply p-2 rounded bg-white/[0.02] border border-white/10;
}

.assistant-content {
  @apply flex-1 flex flex-col p-4 gap-4 overflow-y-auto;
}

.toggle-section {
  @apply flex justify-center;
}

.toggle-button {
  @apply flex items-center gap-2 px-4 py-2 rounded-xl font-medium;
  @apply bg-blue-500/20 hover:bg-blue-500/30 text-blue-400 hover:text-blue-300;
  @apply border border-blue-500/30 hover:border-blue-500/50;
  @apply transition-all duration-200;
}

.toggle-button.active {
  @apply bg-green-500/20 hover:bg-green-500/30 text-green-400 hover:text-green-300;
  @apply border-green-500/30 hover:border-green-500/50;
}

.quick-actions-grid {
  @apply grid grid-cols-2 gap-3;
}

.quick-action {
  @apply flex flex-col items-center gap-2 p-4 rounded-xl;
  @apply bg-white/[0.03] hover:bg-white/[0.06];
  @apply border border-white/10 hover:border-white/20;
  @apply transition-all duration-200 active:scale-95;
}

.action-green {
  @apply hover:border-green-500/30 hover:bg-green-500/10;
}

.action-blue {
  @apply hover:border-blue-500/30 hover:bg-blue-500/10;
}

.action-purple {
  @apply hover:border-purple-500/30 hover:bg-purple-500/10;
}

.action-orange {
  @apply hover:border-orange-500/30 hover:bg-orange-500/10;
}

.quick-action.copied {
  @apply bg-green-500/20 border-green-500/50 text-green-400;
}

.action-text {
  @apply text-sm font-medium text-white/80;
  @apply text-center leading-tight;
}

.recommendations-area {
  @apply flex-1 flex flex-col min-h-[200px] bg-white/[0.02] rounded-xl p-3;
  @apply border border-white/10;
}

.top-recommendations {
  @apply grid grid-cols-1 gap-3 mb-3;
}

.top-recommendation {
  @apply p-4 rounded-xl bg-white/[0.05] hover:bg-white/[0.08] border border-white/10 hover:border-white/20 cursor-pointer transition-all duration-200 active:scale-95;
}

.top-recommendation.urgent {
  @apply border-orange-500/40 bg-orange-500/10;
}

.top-recommendation.copied {
  @apply bg-green-500/20 border-green-500/40;
}

.top-recommendation-text {
  @apply text-base text-white/90 leading-relaxed;
}

.top-recommendation-meta {
  @apply flex items-center justify-between text-xs text-white/50 mt-2;
}

.processing {
  @apply flex items-center justify-center gap-2 py-8 text-blue-400;
}

.suggestions-header {
  @apply flex items-center justify-between mb-3 pb-2 border-b border-white/10;
}

.suggestions-title {
  @apply text-sm font-semibold text-white/90;
}

.suggestions-count {
  @apply text-xs text-white/50 bg-white/10 px-2 py-1 rounded-full;
}

.suggestions-list {
  @apply flex flex-col gap-2;
}

.suggestion {
  @apply p-3 rounded-lg bg-white/[0.03] hover:bg-white/[0.06];
  @apply border border-white/10 hover:border-white/20;
  @apply cursor-pointer transition-all duration-200 active:scale-95;
}

.suggestion.urgent {
  @apply border-orange-500/40 bg-orange-500/10;
  animation: gentle-pulse 2s infinite;
}

.suggestion.copied {
  @apply bg-green-500/20 border-green-500/40;
}

@keyframes gentle-pulse {
  0%, 100% { border-color: rgba(251, 146, 60, 0.4); }
  50% { border-color: rgba(251, 146, 60, 0.6); }
}

.suggestion-content {
  @apply flex flex-col gap-2;
}

.suggestion-text {
  @apply text-sm text-white/90 leading-relaxed;
  font-size: 13px;
}

.suggestion-meta {
  @apply flex items-center justify-between text-xs text-white/50;
}

.suggestion-type {
  @apply capitalize;
}

.ai-response {
  @apply p-3 rounded-lg bg-blue-500/10 border border-blue-500/30;
}

.response-header {
  @apply flex items-center gap-2 mb-2;
}

.response-title {
  @apply text-sm font-medium text-blue-400 flex-1;
}

.copy-btn {
  @apply p-1 rounded hover:bg-white/10 text-white/40 hover:text-white/80;
  @apply transition-colors;
}

.copy-btn.copied {
  @apply text-green-400;
}

.response-text {
  @apply text-xs text-white/80 leading-relaxed whitespace-pre-wrap;
  font-size: 12px;
  line-height: 1.5;
}

.empty-state {
  @apply flex-1 flex flex-col items-center justify-center py-8 text-center;
}

.empty-icon {
  @apply mb-3;
}

.empty-title {
  @apply text-sm font-medium text-white/70 mb-1;
}

.empty-subtitle {
  @apply text-xs text-white/40;
}

.error-display {
  @apply flex items-center gap-2 p-3 rounded-lg;
  @apply bg-red-500/10 border border-red-500/30 text-red-400;
}

/* Responsive adjustments */
@media (max-height: 700px) {
  .quick-actions-grid {
    @apply grid-cols-1 gap-2;
  }
  
  .quick-action {
    @apply flex-row justify-start px-3 py-2;
  }
  
  .recommendations-area {
    @apply min-h-[150px];
  }
  
  .template-grid {
    @apply grid-cols-1;
  }
}
</style>