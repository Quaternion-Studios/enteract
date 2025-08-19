<script setup lang="ts">
import { ref, watch, nextTick, toRef, onMounted, onUnmounted, computed } from 'vue'
import {
  CommandLineIcon,
  XMarkIcon,
  ShieldCheckIcon,
  ExclamationTriangleIcon,
  MicrophoneIcon,
  StopIcon,
  QueueListIcon
} from '@heroicons/vue/24/outline'
import { useChatManagement } from '../../composables/useChatManagement'
import { useSpeechEvents } from '../../composables/useSpeechEvents'
import { useSpeechTranscription } from '../../composables/useSpeechTranscription'
import { useWindowRegistration } from '../../composables/useWindowRegistry'
import { useRagDocuments } from '../../composables/rag'
import AgentActionButtons from './AgentActionButtons.vue'
import ModelSelector from './ModelSelector.vue'
import ChatWindowSidebarAdapter from './ChatWindowSidebarAdapter.vue'
import DocumentContextDropdown from '../rag/DocumentContextDropdown.vue'
import DocumentPillsContainer from './DocumentPillsContainer.vue'
import { FileService } from '../../composables/fileService'

interface Props {
  showChatWindow: boolean
  selectedModel: string | null
}

interface Emits {
  (e: 'close'): void
  (e: 'update:showChatWindow', value: boolean): void
  (e: 'update:selectedModel', value: string): void
  (e: 'toggle-chat-drawer'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Refs for scrolling
const chatMessages = ref<HTMLElement>()

// Chat sidebar state
const showChatSidebar = ref(false)

// RAG system
const ragDocuments = useRagDocuments()
const showDocumentDropdown = ref(false)
const documentDropdownPosition = ref({ x: 0, y: 0 })
const documentSearchQuery = ref('')

// Upload states (added for document upload functionality)
const isDragOver = ref(false)
const isUploading = ref(false)
const fileInputRef = ref<HTMLInputElement>()

// Agent and model selection state
const currentAgent = ref('enteract')
const showMentionSuggestions = ref(false)
const mentionSuggestions = ref<Array<{id: string, name: string, description: string}>>([])
const mentionStartPos = ref(-1)

// Available agents for @ mentions
const availableAgents = [
  { id: 'enteract', name: '@enteract', description: 'MCP computer use agent with tool access' },
  { id: 'coding', name: '@coding', description: 'Programming assistance and code review' },
  { id: 'research', name: '@research', description: 'Advanced research with step-by-step thinking' },
  { id: 'vision', name: '@vision', description: 'Visual content analysis' }
]

// MCP mode detection - only highlight when actively typing @enteract
const isMCPMode = computed(() => {
  return chatMessage.value.trim().toLowerCase().startsWith('@enteract')
})

// Handle mention input: '@' for agent/model, '/' for documents
const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement
  const value = target.value
  const cursorPos = target.selectionStart || 0
  
  const beforeCursor = value.substring(0, cursorPos)

  // Document context with '/'
  const lastSlashIndex = beforeCursor.lastIndexOf('/')
  if (lastSlashIndex !== -1) {
    const afterSlash = beforeCursor.substring(lastSlashIndex + 1)
    if (!afterSlash.includes(' ') && afterSlash.length >= 0) {
      const rect = target.getBoundingClientRect()
      documentDropdownPosition.value = { x: rect.left, y: rect.bottom + 5 }
      documentSearchQuery.value = '/' + afterSlash
      showDocumentDropdown.value = true
      showMentionSuggestions.value = false
      mentionStartPos.value = lastSlashIndex
      return
    }
  }

  // Agent/model mentions with '@'
  const lastAtIndex = beforeCursor.lastIndexOf('@')
  if (lastAtIndex !== -1) {
    const afterAt = beforeCursor.substring(lastAtIndex + 1)
    if (!afterAt.includes(' ') && afterAt.length >= 0) {
      const filtered = availableAgents.filter(agent => agent.name.toLowerCase().includes(('@' + afterAt).toLowerCase()))
      if (filtered.length > 0) {
        mentionSuggestions.value = filtered
        mentionStartPos.value = lastAtIndex
        showMentionSuggestions.value = true
        showDocumentDropdown.value = false
        return
      }
    }
  }

  showMentionSuggestions.value = false
  showDocumentDropdown.value = false
}

// Ref for the chat input element
const chatInputRef = ref<HTMLInputElement>()

// Select mention suggestion
const selectMention = (agent: {id: string, name: string, description: string}) => {
  const input = chatInputRef.value
  if (input && mentionStartPos.value !== -1) {
    const beforeMention = chatMessage.value.substring(0, mentionStartPos.value)
    const afterCursor = chatMessage.value.substring(input.selectionStart || 0)
    
    chatMessage.value = beforeMention + agent.name + ' ' + afterCursor
    currentAgent.value = agent.id
    showMentionSuggestions.value = false
    
    // Focus input and position cursor after mention
    setTimeout(() => {
      input.focus()
      const newPos = beforeMention.length + agent.name.length + 1
      input.setSelectionRange(newPos, newPos)
    }, 0)
  }
}

// Parse message for agent mentions before sending
const parseAgentFromMessage = (message: string): string => {
  const mentionMatch = message.match(/@(\w+)/)
  if (mentionMatch) {
    const mentionedAgent = mentionMatch[1]
    const agent = availableAgents.find(a => a.id === mentionedAgent)
    if (agent) {
      currentAgent.value = agent.id
      // Remove the mention from the message
      return message.replace(/@\w+\s*/, '').trim()
    }
  }
  return message
}

// Enhanced keyboard handler
const handleEnhancedKeydown = (event: KeyboardEvent) => {
  if (showDocumentDropdown.value) {
    if (event.key === 'Escape') {
      handleDocumentDropdownClose()
      return
    }
  }
  
  if (showMentionSuggestions.value) {
    if (event.key === 'Escape') {
      showMentionSuggestions.value = false
      return
    }
    if (event.key === 'Tab' || event.key === 'Enter') {
      event.preventDefault()
      if (mentionSuggestions.value.length > 0) {
        selectMention(mentionSuggestions.value[0])
      }
      return
    }
  }
  
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    sendMessageWithAgent()
  }
}

// File upload handlers for chat context
const handleFileUploadInput = async (event: Event) => {
  const input = event.target as HTMLInputElement
  const files = input.files
  if (!files) return
  
  isUploading.value = true
  try {
    // Upload with chat context - auto-selects and enforces limit
    const uploadedDocs = await ragDocuments.uploadDocuments(files, { 
      source: 'chat',
      autoSelect: true,
      maxSelection: ragDocuments.CHAT_DOCUMENT_LIMIT 
    })
    console.log(`📚 Uploaded ${uploadedDocs.length} documents via ChatWindow`)
    
    // Check if we're at the limit
    const limitInfo = ragDocuments.getSelectionLimitInfo()
    if (limitInfo.isAtLimit) {
      console.info(`📑 Document limit reached: ${limitInfo.current}/${limitInfo.max} documents selected`)
    }
  } catch (error) {
    console.error('Failed to upload documents:', error)
  } finally {
    isUploading.value = false
    input.value = ''
  }
}

const handleDragOver = (event: DragEvent) => {
  event.preventDefault()
  isDragOver.value = true
}

const handleDragLeave = () => {
  isDragOver.value = false
}

const handleDrop = async (event: DragEvent) => {
  event.preventDefault()
  isDragOver.value = false
  
  const files = event.dataTransfer?.files
  if (!files) return
  
  isUploading.value = true
  try {
    // Upload with chat context - auto-selects and enforces limit
    const uploadedDocs = await ragDocuments.uploadDocuments(files, { 
      source: 'chat',
      autoSelect: true,
      maxSelection: ragDocuments.CHAT_DOCUMENT_LIMIT 
    })
    console.log(`📚 Uploaded ${uploadedDocs.length} documents via ChatWindow drag & drop`)
    
    // Check if we're at the limit
    const limitInfo = ragDocuments.getSelectionLimitInfo()
    if (limitInfo.isAtLimit) {
      console.info(`📑 Document limit reached: ${limitInfo.current}/${limitInfo.max} documents selected`)
    }
  } catch (error) {
    console.error('Failed to upload documents:', error)
  } finally {
    isUploading.value = false
  }
}

// Enhanced send message with agent detection and RAG context
const sendMessageWithAgent = async () => {
  if (!chatMessage.value.trim()) return
  
  let originalMessage = chatMessage.value.trim()
  const cleanedMessage = parseAgentFromMessage(originalMessage)
  
  // Prepare selected document IDs for RAG search
  const selectedDocIds = Array.from(ragDocuments.selectedDocumentIds.value)
  
  if (selectedDocIds.length > 0) {
    console.log(`📚 Sending message with ${selectedDocIds.length} selected documents for RAG context`)
  }
  
  // Store current agent before clearing the form
  const agentToUse = currentAgent.value
  
  chatMessage.value = ''
  showMentionSuggestions.value = false
  showDocumentDropdown.value = false
  
  // Reset to default agent unless the message explicitly mentioned an agent
  const hasExplicitAgentMention = originalMessage.toLowerCase().match(/@(enteract|coding|research|vision)\b/)
  if (!hasExplicitAgentMention) {
    currentAgent.value = 'enteract'
  }
  
  // For MCP commands, always pass the original message to preserve @enteract prefix
  const messageToSend = originalMessage.toLowerCase().startsWith('@enteract') ? originalMessage : (cleanedMessage || originalMessage)
  await sendMessage(agentToUse, messageToSend, selectedDocIds)
}

// Handle model and agent selection
const handleModelUpdate = (modelName: string) => {
  emit('update:selectedModel', modelName)
}

const handleAgentUpdate = (agentId: string) => {
  currentAgent.value = agentId
}

// Helper function to scroll chat to bottom
const scrollChatToBottom = () => {
  if (chatMessages.value) {
    chatMessages.value.scrollTop = chatMessages.value.scrollHeight
  }
}

// Chat management composable
const {
  chatMessage,
  chatHistory,
  createNewChat,
  switchChat,
  deleteChat,
  clearChat,
  fileInput,
  renderMarkdown,
  takeScreenshotAndAnalyze,
  startDeepResearch,
  startConversationalAgent,
  startCodingAgent,
  startComputerUseAgent,
  sendMessage,
  triggerFileUpload,
  handleFileUpload,
  estimateTokens,
  cancelResponse
} = useChatManagement(props.selectedModel, scrollChatToBottom, currentAgent)

// Context truncation detection
const MAX_TOKENS = 4000
const isContextTruncated = computed(() => {
  if (!chatHistory.value || chatHistory.value.length === 0) return false
  
  const totalTokens = chatHistory.value.reduce((sum, message) => {
    return sum + estimateTokens(message.text)
  }, 0)
  
  return totalTokens > MAX_TOKENS
})


// Set up speech events with real chat management functions
const { setupSpeechTranscriptionListeners, removeSpeechTranscriptionListeners } = useSpeechEvents(
  chatHistory,
  toRef(props, 'showChatWindow'),
  scrollChatToBottom,
  chatMessage,
  (agentType?: string) => sendMessage(agentType || currentAgent.value)
)

// Window resizing composable - not currently used in this component
// const {
//   chatWindowSize,
//   isResizing,
//   startResize
// } = useWindowResizing()

// Speech transcription for microphone button
const {
  initialize: initializeSpeech,
  startRecording: startSpeechRecording,
  stopRecording: stopSpeechRecording,
  isRecording: isSpeechRecording,
  isInitialized: isSpeechInitialized,
  setAutoSendToChat,
  setContinuousMode
} = useSpeechTranscription()

// Window registry for centralized window management
const windowRegistry = useWindowRegistration('chat-window', {
  closeOnClickOutside: false, // Temporarily disabled for testing
  isModal: false,
  priority: 200, // Higher than settings panel
  closeHandler: () => closeWindow()
})

const closeWindow = () => {
  emit('close')
  emit('update:showChatWindow', false)
}

// Ref for the chat window element
const chatWindowRef = ref<HTMLElement>()


// Focus input when chat window opens and register/unregister with window registry
watch(() => props.showChatWindow, async (newValue) => {
  if (newValue) {
    await nextTick()
    
    // Register the window element when it opens
    if (chatWindowRef.value) {
      windowRegistry.registerSelf(chatWindowRef.value)
    }
    
    setTimeout(() => {
      const input = chatInputRef.value
      if (input) input.focus()
    }, 150)
  } else {
    // Unregister when window closes
    windowRegistry.unregisterSelf()
  }
})

// Auto-scroll when chat history changes
watch(chatHistory, async () => {
  await nextTick()
  scrollChatToBottom()
}, { deep: true })

// Agent action handlers
const handleTakeScreenshot = () => {
  takeScreenshotAndAnalyze({ value: props.showChatWindow })
}

const handleStartDeepResearch = () => {
  startDeepResearch({ value: props.showChatWindow })
}

const handleStartConversational = () => {
  startConversationalAgent({ value: props.showChatWindow })
}

const handleStartCoding = () => {
  startCodingAgent({ value: props.showChatWindow })
}

const handleStartComputerUse = () => {
  startComputerUseAgent({ value: props.showChatWindow })
}


const handleFileUploadEvent = (event: Event) => {
  handleFileUpload(event, { value: props.showChatWindow })
}

// Microphone functionality for chat interface
const handleMicrophoneToggle = async () => {
  try {
    if (isSpeechRecording.value) {
      await stopSpeechRecording()
      console.log('🎤 Chat: Speech recording stopped')
    } else {
      if (!isSpeechInitialized.value) {
        await initializeSpeech()
      }
      await startSpeechRecording()
      console.log('🎤 Chat: Speech recording started')
    }
  } catch (error) {
    console.error('Chat microphone toggle error:', error)
  }
}

// Sidebar functions
const toggleChatSidebar = () => {
  showChatSidebar.value = !showChatSidebar.value
  console.log(`💬 Chat sidebar ${showChatSidebar.value ? 'opened' : 'closed'}`)
}

const handleCreateNewChat = () => {
  createNewChat()
  showChatSidebar.value = false
}

const handleSwitchChat = (chatId: string) => {
  switchChat(chatId)
  showChatSidebar.value = false
}

const handleDeleteChat = (chatId: string) => {
  deleteChat(chatId)
}

const handleClearChat = () => {
  clearChat()
}



// Document handling functions
const handleDocumentSelect = (documentId: string) => {
  ragDocuments.toggleDocumentSelection(documentId, 'chat')
}

const handleDocumentDeselect = (documentId: string) => {
  ragDocuments.toggleDocumentSelection(documentId, 'chat')
}

const handleDocumentUpload = async (files: FileList) => {
  try {
    // Upload with chat context - auto-selects and enforces limit
    const uploadedDocs = await ragDocuments.uploadDocuments(files, { 
      source: 'chat',
      autoSelect: true,
      maxSelection: ragDocuments.CHAT_DOCUMENT_LIMIT 
    })
    console.log(`📚 Uploaded ${uploadedDocs.length} documents via dropdown`)
    
    // Check if we're at the limit
    const limitInfo = ragDocuments.getSelectionLimitInfo()
    if (limitInfo.isAtLimit) {
      console.info(`📑 Document limit reached: ${limitInfo.current}/${limitInfo.max} documents selected`)
    }
    
    // Close dropdown after successful upload
    showDocumentDropdown.value = false
  } catch (error) {
    console.error('Failed to upload documents:', error)
  }
}

const handleInsertReference = (fileName: string) => {
  const input = chatInputRef.value
  if (input && mentionStartPos.value !== -1) {
    const beforeMention = chatMessage.value.substring(0, mentionStartPos.value)
    const afterCursor = chatMessage.value.substring(input.selectionStart || 0)
    
    chatMessage.value = beforeMention + '/' + fileName + ' ' + afterCursor
    showDocumentDropdown.value = false
    
    // Focus input
    nextTick(() => {
      input.focus()
    })
  }
}

// Handle dropdown close to remove '/' if nothing was selected
const handleDocumentDropdownClose = () => {
  if (mentionStartPos.value !== -1 && chatMessage.value.charAt(mentionStartPos.value) === '/') {
    // Check if there's anything after the '/' that looks like a file reference
    const input = chatInputRef.value
    if (input) {
      const beforeMention = chatMessage.value.substring(0, mentionStartPos.value)
      const afterMention = chatMessage.value.substring(mentionStartPos.value + 1)
      
      // If there's only whitespace or nothing after '/', remove the '/'
      if (!afterMention.trim() || afterMention.startsWith(' ')) {
        chatMessage.value = beforeMention + afterMention
      }
    }
  }
  showDocumentDropdown.value = false
  mentionStartPos.value = -1
}

// Setup speech events when component mounts
onMounted(async () => {
  setupSpeechTranscriptionListeners()
  console.log('🎤 ChatWindow: Speech transcription listeners set up')
  
  // Initialize RAG system
  try {
    await ragDocuments.initialize()
    FileService.setRagComposable(ragDocuments)
    console.log('📚 RAG system initialized with', ragDocuments.documents.value.length, 'documents')
  } catch (error) {
    console.error('Failed to initialize RAG system:', error)
  }
  
  // Initialize speech transcription for chat interface
  try {
    await initializeSpeech()
    // Enable auto-send to chat for the chat interface (default behavior)
    setAutoSendToChat(true)
    // Disable continuous mode for chat interface (short recordings)
    setContinuousMode(false)
    console.log('🎤 ChatWindow: Speech transcription initialized for chat interface')
  } catch (error) {
    console.error('🎤 ChatWindow: Failed to initialize speech transcription:', error)
  }
  
  // Scroll to bottom on mount if there are messages
  if (chatHistory.value.length > 0) {
    await nextTick()
    scrollChatToBottom()
  }
})

// Cleanup speech events when component unmounts
onUnmounted(() => {
  removeSpeechTranscriptionListeners()
  console.log('🎤 ChatWindow: Speech transcription listeners removed')
})
</script>

<template>
  <Transition name="chat-window">
    <div 
      v-if="showChatWindow" 
      ref="chatWindowRef" 
      class="chat-window"
      :class="{ 'drag-over': isDragOver }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
    >
      <!-- Window Header -->
      <div class="window-header">
        <div class="header-title">
          <div class="flex items-center gap-2">
            <CommandLineIcon class="w-4 h-4 text-white/80" />
            <span class="text-sm font-medium text-white/90">AI Assistant</span>
            <!-- Model Selector -->
            <ModelSelector
              :selected-model="selectedModel"
              :current-agent="currentAgent"
              @update:selected-model="handleModelUpdate"
              @update:current-agent="handleAgentUpdate"
            />
            
            <!-- Context Truncation Indicator -->
            <div v-if="isContextTruncated" class="truncation-indicator" title="Chat history is being truncated to fit AI context limits">
              <ExclamationTriangleIcon class="w-3 h-3 text-yellow-400" />
              <span class="text-xs text-yellow-400">History Truncated</span>
            </div>
          </div>
          <div class="header-controls">
            <!-- Chat History Button -->
            <button 
              @click="toggleChatSidebar" 
              class="export-btn"
              :class="{ 'active': showChatSidebar }"
              title="Show chat history"
            >
              <QueueListIcon class="w-3 h-3" />
            </button>
          </div>
        </div>
        <button @click="closeWindow" class="close-btn">
          <XMarkIcon class="w-4 h-4 text-white/70 hover:text-white transition-colors" />
        </button>
      </div>
      
      <!-- RAG Context Status -->
      <div v-if="ragDocuments.selectedDocumentIds.value.size > 0" class="rag-context-status">
        <div class="rag-status-content">
          <span class="rag-icon">📚</span>
          <span class="rag-text">
            {{ ragDocuments.selectedDocumentIds.value.size }} document{{ ragDocuments.selectedDocumentIds.value.size !== 1 ? 's' : '' }} selected for context
          </span>
          <button 
            @click="ragDocuments.clearSelection()" 
            class="clear-selection-btn"
            title="Clear document selection"
          >
            ×
          </button>
        </div>
      </div>
      
      <!-- Window Content Container -->
      <div class="window-content">
        <!-- Chat Sidebar -->
        <ChatWindowSidebarAdapter
          :show="showChatSidebar"
          :selected-model="selectedModel"
          @close="showChatSidebar = false"
          @new-chat="handleCreateNewChat"
          @switch-chat="handleSwitchChat"
          @delete-chat="handleDeleteChat"
          @clear-chat="handleClearChat"
        />
        
        <!-- Main Content Area -->
        <div class="main-content" :class="{ 'with-sidebar': showChatSidebar }">
          <!-- Hidden file input for upload documents -->
          <input
            ref="fileInputRef"
            type="file"
            multiple
            accept=".pdf,.txt,.md,.doc,.docx,.rtf"
            @change="handleFileUploadInput"
            class="hidden"
          />
          
          <!-- Chat Messages Area -->
          <div ref="chatMessages" class="chat-area">
            <div v-if="chatHistory.length === 0" class="empty-state">
              <div class="empty-icon">
                <CommandLineIcon class="w-8 h-8 text-white/40" />
              </div>
              <p class="text-white/60 text-sm">Start a conversation</p>
              <p class="text-white/40 text-xs mt-1">
                Ask your AI assistant anything
              </p>
            </div>
            
            <div v-else class="messages-container">
              <div 
                v-for="(message, index) in chatHistory" 
                :key="index"
                class="message-wrapper"
                :class="{
                  'user-message': message.sender === 'user',
                  'assistant-message': message.sender === 'assistant',
                  'transcription-message': message.sender === 'transcription',
                  'system-message': message.sender === 'system'
                }"
              >
                <div class="message-bubble" :class="{
                  'user-bubble': message.sender === 'user',
                  'assistant-bubble': message.sender === 'assistant',
                  'transcription-bubble': message.sender === 'transcription',
                  'system-bubble': message.sender === 'system'
                }">
                  <div class="message-header">
                    <div class="message-source">
                      <span class="source-label">
                        {{ message.sender === 'user' ? 'You' : 
                           message.sender === 'assistant' ? 'Assistant' : 
                           message.sender === 'transcription' ? 'Voice' : 'System' }}
                      </span>
                    </div>
                    <span class="message-time">{{ new Date(message.timestamp).toLocaleTimeString() }}</span>
                  </div>
                  <div class="message-content">
                    <!-- Transcription messages -->
                    <div v-if="message.sender === 'transcription'" class="transcription-message">
                      <!-- Interim transcription (thought stream) -->
                      <div v-if="message.isInterim" class="interim-transcription">
                        <span class="interim-icon">💭</span>
                        <span class="interim-text">{{ message.text }}</span>
                        <span class="interim-dots">...</span>
                      </div>
                      <!-- Final transcription -->
                      <div v-else class="final-transcription">
                        <div class="transcription-content">
                          <span class="transcription-icon">🎤</span>
                          <span class="transcription-text">{{ message.text }}</span>
                        </div>
                        <div v-if="message.confidence" class="confidence-indicator">
                          {{ Math.round(message.confidence * 100) }}%
                        </div>
                      </div>
                    </div>
                    
                    <!-- Regular user/assistant messages -->
                    <div v-else>
                      <!-- Streaming text with cursor -->
                      <template v-if="message.text.endsWith('▋')">
                        <div v-html="renderMarkdown(message.text.slice(0, -1))"></div><span class="streaming-cursor">▋</span>
                      </template>
                      <!-- Regular completed text with markdown -->
                      <div v-else v-html="renderMarkdown(message.text)"></div>
                    </div>
                  </div>
                  <div v-if="message.confidence && message.sender !== 'transcription'" class="message-confidence">
                    Confidence: {{ Math.round(message.confidence * 100) }}%
                  </div>
                  <!-- Cancel button for streaming messages -->
                  <div v-if="message.isStreaming && message.sender === 'assistant'" class="message-actions">
                    <button @click="cancelResponse(message.id)" class="cancel-button" title="Cancel response">
                      <StopIcon class="w-4 h-4" />
                      Cancel
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Agent Action Buttons with Upload Docs -->
          <AgentActionButtons
            :file-input="fileInput"
            :is-uploading="isUploading"
            @take-screenshot="handleTakeScreenshot"
            @start-deep-research="handleStartDeepResearch"
            @start-conversational="handleStartConversational"
            @start-coding="handleStartCoding"
            @start-computer-use="handleStartComputerUse"
            @trigger-file-upload="() => fileInputRef?.click()"
            @handle-file-upload="handleFileUploadInput"
          />
          
          <!-- Chat Input -->
          <div class="chat-input-container">
            <div class="input-wrapper">
              <!-- Document Pills Container -->
              <DocumentPillsContainer
                v-if="ragDocuments.selectedDocumentIds.value.size > 0"
                :documents="ragDocuments.documents.value"
                :selected-document-ids="ragDocuments.selectedDocumentIds.value"
                :limit-info="ragDocuments.getSelectionLimitInfo()"
                :max-visible="3"
                @deselect="handleDocumentDeselect"
              />
              <input 
                ref="chatInputRef"
                v-model="chatMessage"
                @input="handleInput"
                @keydown="handleEnhancedKeydown"
                class="chat-input"
                :class="{ 'mcp-mode': isMCPMode }"
                :placeholder="isMCPMode ? 'MCP Mode: Enter computer use command...' : 'Ask any AI agent... (use @ to mention agents, / to add documents)'"
                type="text"
              />
              
              
              <!-- @ Mention Suggestions -->
              <Transition name="mention-suggestions">
                <div v-if="showMentionSuggestions" class="mention-suggestions">
                  <button
                    v-for="agent in mentionSuggestions"
                    :key="agent.id"
                    @click="selectMention(agent)"
                    class="mention-suggestion"
                  >
                    <div class="mention-name">{{ agent.name }}</div>
                    <div class="mention-description">{{ agent.description }}</div>
                  </button>
                </div>
              </Transition>
            </div>
            
            <!-- Microphone Button -->
            <button 
              @click="handleMicrophoneToggle"
              :disabled="!isSpeechInitialized"
              class="chat-mic-btn"
              :class="{
                'recording': isSpeechRecording,
                'disabled': !isSpeechInitialized
              }"
              :title="isSpeechRecording ? 'Stop recording' : 'Start voice input'"
            >
              <StopIcon v-if="isSpeechRecording" class="w-4 h-4" />
              <MicrophoneIcon v-else class="w-4 h-4" />
            </button>
            
            <button @click="sendMessageWithAgent" class="chat-send-btn" :disabled="!chatMessage.trim()">
              <ShieldCheckIcon class="w-4 h-4" />
            </button>
          </div>
        </div> <!-- End main-content -->
      </div> <!-- End window-content -->
      
      <!-- Document Context Dropdown -->
      <DocumentContextDropdown
        :documents="ragDocuments.documents.value"
        :selected-document-ids="ragDocuments.selectedDocumentIds.value"
        :show="showDocumentDropdown"
        :position="documentDropdownPosition"
        :search-query="documentSearchQuery"
        :max-selections="5"
        @select="handleDocumentSelect"
        @deselect="handleDocumentDeselect"
        @insert-reference="handleInsertReference"
        @upload-documents="handleDocumentUpload"
        @close="handleDocumentDropdownClose"
      />
    </div>
  </Transition>
</template>

<style scoped>
.chat-window {
  @apply backdrop-blur-xl border border-white/15 rounded-2xl overflow-hidden;
  background: linear-gradient(to bottom, 
    rgba(10, 10, 12, 0.9) 0%, 
    rgba(5, 5, 7, 0.95) 100%
  );
  width: 800px;
  height: 900px;
  max-width: 95vw;
  max-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
  position: relative;
  
  /* Enhanced glass effect similar to conversational window */
  backdrop-filter: blur(80px) saturate(180%);
  box-shadow: 
    0 20px 60px rgba(0, 0, 0, 0.4),
    0 8px 24px rgba(0, 0, 0, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.3),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1);
}

/* Drag and drop visual feedback */
.chat-window.drag-over {
  border-color: rgba(59, 130, 246, 0.6) !important;
  background: linear-gradient(to bottom, 
    rgba(10, 10, 12, 0.95) 0%, 
    rgba(5, 5, 7, 0.98) 100%
  ) !important;
}

.chat-window.drag-over::after {
  content: '📁 Drop files here to upload to RAG system';
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: rgba(59, 130, 246, 0.9);
  color: white;
  padding: 12px 24px;
  border-radius: 12px;
  font-weight: 600;
  z-index: 1000;
  pointer-events: none;
}

/* When sidebar is shown, make window wider and use row layout */
.chat-window:has(.chat-window-sidebar) {
  width: 1200px;
  max-width: 95vw;
}

.window-content {
  @apply flex-1 flex flex-col min-h-0;
}

.chat-window:has(.chat-window-sidebar) .window-content {
  @apply flex flex-row;
}

.main-content {
  @apply flex-1 flex flex-col min-h-0;
}

.window-header {
  @apply flex items-center justify-between px-4 py-3 border-b border-white/10;
  flex-shrink: 0;
}

.header-title {
  @apply flex items-center justify-between w-full mr-4;
}

.header-controls {
  @apply flex items-center gap-2;
}

.header-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-all duration-200 text-white/60 hover:text-white/90;
}

.header-btn.active {
  @apply bg-blue-500/20 text-blue-400 hover:bg-blue-500/30;
}

/* Agent Actions Container */
.agent-actions-container {
  @apply flex items-center gap-3;
}

/* Upload Docs Button (replaces Upload Files in action area) */
.upload-docs-btn {
  @apply flex items-center gap-2 px-4 py-2 rounded-xl border text-sm font-medium transition-all duration-200;
  background: linear-gradient(135deg, 
    rgba(34, 197, 94, 0.8) 0%, 
    rgba(22, 163, 74, 0.8) 100%
  );
  border: 1px solid rgba(34, 197, 94, 0.4);
  color: white;
}

.upload-docs-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, 
    rgba(34, 197, 94, 0.9) 0%, 
    rgba(22, 163, 74, 0.9) 100%
  );
  border-color: rgba(34, 197, 94, 0.6);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(34, 197, 94, 0.3);
}

.upload-docs-btn:disabled {
  background: rgba(75, 85, 99, 0.8);
  border-color: rgba(75, 85, 99, 0.4);
  color: rgba(255, 255, 255, 0.3);
  cursor: not-allowed;
}

.export-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-all duration-200 text-white/60 hover:text-white/90;
}

.export-btn.active {
  @apply bg-blue-500/20 text-blue-400 hover:bg-blue-500/30;
}

/* RAG Context Status Bar */
.rag-context-status {
  @apply px-4 py-2 border-b border-white/10 flex-shrink-0;
  background: linear-gradient(135deg, 
    rgba(59, 130, 246, 0.1) 0%, 
    rgba(99, 102, 241, 0.1) 100%
  );
}

.rag-status-content {
  @apply flex items-center justify-between text-sm;
  color: rgba(255, 255, 255, 0.85);
}

.rag-icon {
  @apply mr-2 text-base;
}

.rag-text {
  @apply font-medium flex-1;
}

.clear-selection-btn {
  @apply ml-3 px-2 py-1 rounded hover:bg-white/10 transition-colors text-white/60 hover:text-white/90;
  font-size: 16px;
  line-height: 1;
}

/* Enhanced drag feedback for RAG status */
.chat-window.drag-over .rag-context-status {
  background: linear-gradient(135deg, 
    rgba(59, 130, 246, 0.2) 0%, 
    rgba(99, 102, 241, 0.2) 100%
  );
  border-color: rgba(59, 130, 246, 0.4);
}

.model-indicator {
  @apply flex items-center px-2 py-1 bg-green-500/20 rounded-full border border-green-500/30;
}

.truncation-indicator {
  @apply flex items-center gap-1 px-2 py-1 bg-yellow-500/20 rounded-full border border-yellow-500/30;
}

.close-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-colors;
}

.empty-state {
  @apply flex flex-col items-center justify-center h-full text-center p-8;
}

.empty-icon {
  @apply mb-4 p-4 rounded-full bg-white/5;
}



/* Chat Area */
.chat-area {
  @apply flex-1 overflow-y-auto;
  min-height: 0;
}

.messages-container {
  @apply p-4 space-y-4;
}

.message-wrapper {
  @apply flex;
}

.message-wrapper.user-message {
  @apply justify-end;
}

.message-wrapper.assistant-message,
.message-wrapper.transcription-message,
.message-wrapper.system-message {
  @apply justify-start;
}

.message-bubble {
  @apply rounded-2xl p-3 w-full;
  word-wrap: break-word;
}

.user-bubble {
  @apply bg-blue-500/80 text-white;
  border-bottom-right-radius: 6px;
}

.assistant-bubble,
.system-bubble {
  @apply text-white/90 border border-white/10;
  background: rgba(255, 255, 255, 0.05);
  border-bottom-left-radius: 6px;
}

.transcription-bubble {
  @apply bg-purple-500/20 text-white/90 border border-purple-400/30;
  border-bottom-left-radius: 6px;
}

.message-header {
  @apply flex items-center justify-between mb-1;
}

.message-source {
  @apply flex items-center gap-1 text-xs opacity-75;
}

.source-label {
  @apply font-medium;
}

.message-time {
  @apply text-xs opacity-60;
}

.message-content {
  @apply text-sm leading-relaxed;
}

.message-confidence {
  @apply text-xs opacity-60 mt-1;
}

.message-actions {
  @apply mt-2 flex items-center gap-2;
}

.cancel-button {
  @apply flex items-center gap-1 px-3 py-1 text-xs bg-red-500/20 hover:bg-red-500/30 text-red-300 rounded-lg transition-colors;
  @apply border border-red-500/30 hover:border-red-500/50;
}

/* Transcription specific styles */
.transcription-message {
  @apply w-full;
}

.interim-transcription {
  @apply flex items-center gap-2;
}

.interim-icon {
  @apply text-orange-300 text-sm;
}

.interim-text {
  @apply italic text-orange-200;
}

.interim-dots {
  @apply text-orange-400 animate-pulse font-bold;
}

.final-transcription {
  @apply flex flex-col gap-1;
}

.transcription-content {
  @apply flex items-center gap-2;
}

.transcription-icon {
  @apply text-green-400 text-sm;
}

.transcription-text {
  @apply text-white/90;
}

.confidence-indicator {
  @apply text-xs text-white/60 mt-1;
}

/* Chat Input */
.chat-input-container {
  @apply flex items-center gap-3 px-4 py-4 border-t border-white/10;
  flex-shrink: 0;
  background: rgba(0, 0, 0, 0.2);
}

.input-wrapper {
  @apply flex-1 relative;
}

.doc-pills {
  @apply absolute -top-8 left-0 right-0 flex flex-wrap gap-2 px-1 pb-1; 
}

.doc-pill {
  @apply inline-flex items-center gap-1 px-2 py-1 rounded-lg text-xs border transition-all duration-200; 
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.12) 0%, rgba(147, 51, 234, 0.12) 100%);
  border-color: rgba(255, 255, 255, 0.15);
  color: rgba(255, 255, 255, 0.85);
}

.doc-pill:hover {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.2) 0%, rgba(147, 51, 234, 0.2) 100%);
  border-color: rgba(255, 255, 255, 0.25);
  transform: translateY(-1px);
}

.pill-text {
  @apply truncate max-w-[180px];
}

.pill-close {
  @apply ml-1 text-white/60 hover:text-white transition-colors;
}

.chat-input {
  @apply w-full border border-white/10 rounded-xl px-4 py-3 text-white placeholder-white/50 focus:outline-none focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/20 transition-all duration-200;
  background: rgba(255, 255, 255, 0.05);
  backdrop-filter: blur(10px);
  font-size: 14px;
}

.chat-input:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.2);
}

.chat-input:focus {
  background: rgba(255, 255, 255, 0.1);
}

/* @ Mention Suggestions */
.mention-suggestions {
  @apply absolute bottom-full left-0 right-0 mb-2 bg-black/95 border border-white/20 rounded-xl shadow-xl backdrop-blur-sm z-50 max-h-32 overflow-y-auto;
}

.mention-suggestion {
  @apply w-full p-3 text-left hover:bg-white/10 transition-colors duration-200 border-b border-white/10 last:border-b-0;
}

.mention-suggestion:first-child {
  @apply rounded-t-xl;
}

.mention-suggestion:last-child {
  @apply rounded-b-xl;
}

.mention-name {
  @apply text-sm font-medium text-blue-400;
}

.mention-description {
  @apply text-xs text-white/60 mt-0.5;
}

/* Mention Suggestions Transition */
.mention-suggestions-enter-active,
.mention-suggestions-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.mention-suggestions-enter-from {
  opacity: 0;
  transform: translateY(10px) scale(0.95);
}

.mention-suggestions-leave-to {
  opacity: 0;
  transform: translateY(10px) scale(0.95);
}

/* Chat Microphone Button */
.chat-mic-btn {
  @apply rounded-xl p-3 transition-all duration-200 flex items-center justify-center;
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.8), rgba(22, 163, 74, 0.8));
  border: 1px solid rgba(34, 197, 94, 0.4);
  color: white;
  min-width: 44px;
  min-height: 44px;
}

.chat-mic-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.9), rgba(22, 163, 74, 0.9));
  border-color: rgba(34, 197, 94, 0.6);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(34, 197, 94, 0.3);
}

.chat-mic-btn.recording {
  @apply animate-pulse;
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.8), rgba(220, 38, 38, 0.8));
  border-color: rgba(239, 68, 68, 0.6);
}

.chat-mic-btn.recording:hover {
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.9), rgba(220, 38, 38, 0.9));
  border-color: rgba(239, 68, 68, 0.7);
}

.chat-mic-btn.disabled {
  background: rgba(75, 85, 99, 0.8);
  border-color: rgba(75, 85, 99, 0.4);
  color: rgba(255, 255, 255, 0.3);
  cursor: not-allowed;
}

.chat-send-btn {
  @apply rounded-xl p-3 transition-all duration-200 flex items-center justify-center;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.8), rgba(37, 99, 235, 0.8));
  border: 1px solid rgba(59, 130, 246, 0.4);
  color: white;
  min-width: 44px;
  min-height: 44px;
}

.chat-send-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.9), rgba(37, 99, 235, 0.9));
  border-color: rgba(59, 130, 246, 0.6);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
}

.chat-send-btn:disabled {
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.3);
  cursor: not-allowed;
}

/* Streaming cursor */
.streaming-cursor {
  animation: blink 1s infinite;
  color: #60a5fa;
  font-weight: bold;
}

@keyframes blink {
  0%, 50% {
    opacity: 1;
  }
  51%, 100% {
    opacity: 0;
  }
}

/* Transitions */
.chat-window-enter-active,
.chat-window-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.chat-window-enter-from {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}

.chat-window-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}

/* MCP Mode Styles */
.chat-input.mcp-mode {
  border-color: rgba(249, 115, 22, 0.6) !important;
  background: rgba(249, 115, 22, 0.1) !important;
  box-shadow: 0 0 0 2px rgba(249, 115, 22, 0.2) !important;
}

/* Scrollbar */
.chat-area::-webkit-scrollbar,
.chat-list::-webkit-scrollbar {
  width: 4px;
}

.chat-area::-webkit-scrollbar-track,
.chat-list::-webkit-scrollbar-track {
  background: transparent;
}

.chat-area::-webkit-scrollbar-thumb,
.chat-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}
</style>