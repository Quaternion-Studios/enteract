<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, nextTick } from 'vue'
import { 
  MicrophoneIcon, 
  SpeakerWaveIcon, 
  XMarkIcon,
  QueueListIcon,
  PencilIcon,
  SparklesIcon,
  RocketLaunchIcon
} from '@heroicons/vue/24/outline'
import { useSpeechTranscription } from '../../composables/useSpeechTranscription'
import { useConversationStore } from '../../stores/conversation'
import { useWindowResizing } from '../../composables/useWindowResizing'
import { useWindowRegistration } from '../../composables/useWindowRegistry'
import { useLiveAI } from '../../composables/useLiveAI'
import { invoke } from '@tauri-apps/api/core'

// Components
import MessageList from '../conversational/MessageList.vue'
import ConversationSidebarAdapter from '../conversational/ConversationSidebarAdapter.vue'
import AIAssistant from '../conversational/AIAssistant.vue'
import LiveAI from '../conversational/LiveAI.vue'
import ExportControls from '../conversational/ExportControls.vue'

// Composables
import { useLoopbackTranscription } from '../../composables/useLoopbackTranscription'
import { useConversationManagement } from '../../composables/useConversationManagement'
import { useAIAssistant } from '../../composables/useAIAssistant'

interface Props {
  showConversationalWindow: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'update:showConversationalWindow', value: boolean): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Store and composables
const conversationStore = useConversationStore()
const { resizeWindow } = useWindowResizing()

// Window registry for centralized window management
const windowRegistry = useWindowRegistration('conversational-window', {
  closeOnClickOutside: false, // Temporarily disabled for testing
  isModal: false,
  priority: 150, // Lower than chat window
  closeHandler: () => closeWindow()
})

// Components refs
const messageListRef = ref<InstanceType<typeof MessageList>>()
const conversationalWindowRef = ref<HTMLElement>()

// State
const audioLoopbackDeviceId = ref<string | null>(null)
const selectedMessages = ref<Set<string>>(new Set())
const showExportControls = ref(false)
const isSaving = ref(false) // Visual indicator for save operations

// Sidebar and panel states
const showConversationSidebar = ref(false)
const showAIAssistant = ref(false)
const showLiveAI = ref(false)

// Speech transcription
const {
  initialize: initializeSpeech,
  startRecording,
  stopRecording,
  isRecording,
  isInitialized: isSpeechInitialized,
  error: speechError,
  setAutoSendToChat,
  setContinuousMode
} = useSpeechTranscription()

// Loopback transcription composable
const {
  isLoopbackTyping,
  loopbackPreviewMessage,
  currentPreviewMessageId,
  isMicrophoneTyping,
  microphonePreviewMessage,
  currentMicPreviewMessageId,
  setupLoopbackListeners,
  cleanupLoopback
} = useLoopbackTranscription()

// Conversation management
const {
  allConversations,
  isLoadingConversations,
  loadConversations,
  createNewConversation,
  resumeConversation,
  deleteConversation
} = useConversationManagement()

// AI Assistant
const {
  aiResponse,
  aiIsProcessing,
  aiError,
  queryAI
} = useAIAssistant()

// Live AI
const {
  isActive: isLiveAIActive,
  response: liveAIResponse,
  suggestions: liveAISuggestions,
  isProcessing: liveAIIsProcessing,
  error: liveAIError,
  startLiveAI,
  stopLiveAI,
  onConversationChange
} = useLiveAI()

// Computed
const messages = computed(() => {
  try {
    const baseMessages = conversationStore.currentMessages || []
    const messagesWithPreviews = [...baseMessages]
    
    // Add loopback typing preview if active
    if (isLoopbackTyping.value && loopbackPreviewMessage.value.trim()) {
      messagesWithPreviews.push({
        id: currentPreviewMessageId.value || 'loopback-preview',
        type: 'system',
        source: 'loopback',
        content: loopbackPreviewMessage.value,
        confidence: 0.5,
        timestamp: Date.now(),
        isPreview: true,
        isTyping: true
      })
    }
    
    // Add microphone typing preview if active
    if (isMicrophoneTyping.value && microphonePreviewMessage.value.trim()) {
      messagesWithPreviews.push({
        id: currentMicPreviewMessageId.value || 'mic-preview',
        type: 'user',
        source: 'microphone',
        content: microphonePreviewMessage.value,
        confidence: 0.5,
        timestamp: Date.now(),
        isPreview: true,
        isTyping: true
      })
    }
    
    return messagesWithPreviews
  } catch (error) {
    console.error('❌ Error computing messages:', error)
    return []
  }
})

const isAudioLoopbackActive = computed(() => conversationStore.isAudioLoopbackActive)
const hasSelectedMessages = computed(() => selectedMessages.value.size > 0)

// Watch for window open/close to register/unregister with window registry
watch(() => props.showConversationalWindow, async (newValue) => {
  if (newValue) {
    await nextTick()
    
    // Register the window element when it opens
    if (conversationalWindowRef.value) {
      windowRegistry.registerSelf(conversationalWindowRef.value)
    }
  } else {
    // Unregister when window closes
    windowRegistry.unregisterSelf()
  }
})

// Watch for sidebar changes to resize window
watch(showConversationSidebar, async (newValue) => {
  try {
    await resizeWindow(false, false, false, true, newValue)
  } catch (error) {
    console.error('❌ Failed to resize window for sidebar:', error)
  }
})

// Watch for new messages to trigger continuous live AI response assistance
watch(messages, async (newMessages, oldMessages) => {
  if (!isLiveAIActive.value || !newMessages.length) return
  
  // Check if any new messages were added (user or system)
  const hasNewMessages = !oldMessages || newMessages.length > oldMessages.length
  
  if (hasNewMessages) {
    console.log('💬 New messages detected, updating response assistance')
    await onConversationChange(newMessages)
  }
}, { deep: true })

// Initialize when component mounts
onMounted(async () => {
  try {
    // Initialize speech transcription
    await initializeSpeech()
    setAutoSendToChat(false)
    setContinuousMode(true)
    
    // Load audio loopback settings
    await loadAudioLoopbackSettings()
    
    // Set up event listeners
    setupLoopbackListeners(isRecording)
    
    // Load existing conversations
    console.log('📁 ConversationalWindow: Loading conversations on mount')
    await loadConversations()
    
  } catch (error) {
    console.error('Failed to initialize conversational window:', error)
  }
})

// Cleanup on unmount
onUnmounted(() => {
  cleanupLoopback()
})

// Audio loopback settings
const loadAudioLoopbackSettings = async () => {
  try {
    const settings = await invoke<{
      selectedLoopbackDevice: string | null
      loopbackEnabled: boolean
    } | null>('load_audio_settings')
    
    if (settings?.selectedLoopbackDevice) {
      audioLoopbackDeviceId.value = settings.selectedLoopbackDevice
    }
  } catch (error) {
    console.error('Failed to load audio loopback settings:', error)
  }
}

// Start/stop audio loopback capture
const startAudioLoopbackCapture = async () => {
  if (!audioLoopbackDeviceId.value) return
  
  try {
    await invoke('start_audio_loopback_capture', {
      deviceId: audioLoopbackDeviceId.value
    })
    conversationStore.setAudioLoopbackState(true)
  } catch (error) {
    console.error('Failed to start audio loopback capture:', error)
  }
}

const stopAudioLoopbackCapture = async () => {
  try {
    console.log('🛑 Starting loopback audio cleanup...')
    
    // Parallel cleanup of loopback system
    const cleanupPromises = [
      // Stop loopback capture
      (async () => {
        await invoke('stop_audio_loopback_capture')
        console.log('✅ Loopback capture stopped')
      })(),
      
      // Cleanup loopback whisper context
      (async () => {
        try {
          await invoke('cleanup_whisper_loopback_context')
          console.log('✅ Loopback whisper context cleaned up')
        } catch (whisperError) {
          console.warn('⚠️ Loopback whisper cleanup failed:', whisperError)
          // Continue anyway as this is not critical
        }
      })(),
      
      // Clear audio buffer and release device handles
      (async () => {
        try {
          await invoke('clear_loopback_audio_buffers')
          console.log('✅ Loopback audio buffers cleared')
        } catch (bufferError) {
          console.warn('⚠️ Buffer cleanup failed:', bufferError)
          // Continue anyway
        }
      })()
    ]
    
    // Execute cleanup with timeout
    await Promise.race([
      Promise.allSettled(cleanupPromises),
      new Promise((_, reject) => 
        setTimeout(() => reject(new Error('Loopback cleanup timeout')), 3000)
      )
    ])
    
    // Update state after successful cleanup
    conversationStore.setAudioLoopbackState(false)
    console.log('✅ Loopback audio system cleanup completed')
    
  } catch (error) {
    console.error('❌ Failed to stop audio loopback capture:', error)
    
    // Emergency cleanup - force state reset even if cleanup fails
    conversationStore.setAudioLoopbackState(false)
    
    try {
      await invoke('force_cleanup_whisper_contexts')
      console.log('✅ Emergency loopback cleanup completed')
    } catch (emergencyError) {
      console.error('❌ Emergency loopback cleanup failed:', emergencyError)
    }
  }
}

// Ultra-responsive microphone toggle with instant shutdown
const toggleMicrophone = async () => {
  if (isRecording.value) {
    // INSTANT UI RESPONSE - Reset button state immediately (under 50ms)
    console.log('🛑 INSTANT STOP - UI reset immediately for maximum responsiveness')
    
    try {
      // PHASE 1: Immediate cancellation signals (fire and forget - non-blocking)
      const instantCancellation = async () => {
        try {
          // Send cancellation signals in parallel without waiting
          await Promise.all([
            invoke('cancel_microphone_transcription').catch(console.warn),
            invoke('cancel_loopback_transcription').catch(console.warn)
          ])
          console.log('✅ Instant cancellation signals sent')
        } catch (error) {
          console.warn('⚠️ Cancellation signals failed, but continuing:', error)
        }
      }
      
      // Start cancellation immediately (non-blocking)
      instantCancellation()
      
      // PHASE 2: Background cleanup with progressive timeouts
      const backgroundCleanup = async () => {
        console.log('🧹 Starting background cleanup...')
        
        // Progressive cleanup with model-aware timeouts
        const cleanupTasks = [
          // Audio systems cleanup (should be fast)
          (async () => {
            try {
              await Promise.race([
                Promise.allSettled([
                  stopRecording(),
                  stopAudioLoopbackCapture()
                ]),
                new Promise((_, reject) => 
                  setTimeout(() => reject(new Error('Audio cleanup timeout')), 2000)
                )
              ])
              console.log('✅ Audio systems cleaned up')
            } catch (audioError) {
              console.warn('⚠️ Audio cleanup timeout, force stopping:', audioError)
              // Force reset audio state even if cleanup fails
              isRecording.value = false
              conversationStore.setAudioLoopbackState(false)
            }
          })(),
          
          // Session save (critical - must complete)
          (async () => {
            if (conversationStore.currentSession) {
              const sessionId = conversationStore.currentSession.id
              console.log('💾 Critical: Force saving session:', sessionId)
              
              // Show saving indicator
              isSaving.value = true
              
              try {
                await Promise.race([
                  conversationStore.endSession(),
                  new Promise((_, reject) => 
                    setTimeout(() => reject(new Error('Session save timeout')), 5000)
                  )
                ])
                console.log('✅ Session saved successfully')
                
                // Refresh conversations in background (non-critical)
                loadConversations().catch(console.warn)
                
              } catch (saveError) {
                console.error('❌ Critical: Session save failed:', saveError)
                // Try emergency save
                try {
                  await conversationStore.endSession()
                  console.log('✅ Emergency session save successful')
                } catch (emergencyError) {
                  console.error('❌ Emergency session save also failed:', emergencyError)
                }
              } finally {
                // Hide saving indicator
                isSaving.value = false
              }
            }
          })()
        ]
        
        // Execute all cleanup tasks with overall timeout
        try {
          await Promise.race([
            Promise.allSettled(cleanupTasks),
            new Promise((_, reject) => 
              setTimeout(() => reject(new Error('Overall cleanup timeout')), 8000)
            )
          ])
          console.log('✅ Background cleanup completed successfully')
        } catch (overallTimeout) {
          console.warn('⚠️ Background cleanup timed out, emergency protocol:', overallTimeout)
          
          // Emergency protocol - reset everything
          try {
            await invoke('emergency_shutdown_whisper')
            isRecording.value = false
            conversationStore.setAudioLoopbackState(false)
            console.log('✅ Emergency protocol completed')
          } catch (emergencyError) {
            console.error('❌ Emergency protocol failed:', emergencyError)
            // Last resort - force reset UI state
            isRecording.value = false
            conversationStore.setAudioLoopbackState(false)
          }
        }
      }
      
      // Start background cleanup immediately (don't wait for it)
      backgroundCleanup()
      
      // PHASE 3: Show instant feedback to user
      console.log('✅ Instant stop completed - user sees immediate response')
      
    } catch (error) {
      console.error('❌ Error in instant stop sequence:', error)
      
      // Emergency fallback - ensure UI is responsive
      isRecording.value = false
      conversationStore.setAudioLoopbackState(false)
      
      // Try emergency shutdown
      try {
        await invoke('emergency_shutdown_whisper')
        console.log('✅ Emergency fallback completed')
      } catch (emergencyError) {
        console.error('❌ Emergency fallback failed:', emergencyError)
      }
    }
    
  } else {
    // Starting recording - reset cancellation tokens first
    try {
      await invoke('reset_cancellation_tokens')
      console.log('🔄 Cancellation tokens reset for new session')
    } catch (resetError) {
      console.warn('⚠️ Failed to reset cancellation tokens:', resetError)
    }
    
    if (!conversationStore.currentSession) {
      const session = conversationStore.createSession()
      console.log('🆕 ConversationalWindow: Created new session:', session.id)
    } else {
      console.log('🔄 ConversationalWindow: Using existing session:', conversationStore.currentSession.id)
    }
    
    // Start both systems in parallel with error handling
    const startPromises = [
      startRecording().catch(err => console.error('❌ Microphone start failed:', err))
    ]
    
    if (audioLoopbackDeviceId.value) {
      startPromises.push(
        startAudioLoopbackCapture().catch(err => console.error('❌ Loopback start failed:', err))
      )
    }
    
    await Promise.allSettled(startPromises)
    console.log('✅ Audio systems started')
  }
}

// Export controls
const toggleExportControls = () => {
  showExportControls.value = !showExportControls.value
  if (!showExportControls.value) {
    selectedMessages.value.clear()
  }
}

const toggleMessageSelection = (messageId: string) => {
  if (selectedMessages.value.has(messageId)) {
    selectedMessages.value.delete(messageId)
  } else {
    selectedMessages.value.add(messageId)
  }
}

const selectAllMessages = () => {
  const allMessages = conversationStore.currentMessages || []
  allMessages.forEach(msg => selectedMessages.value.add(msg.id))
}

const deselectAllMessages = () => {
  selectedMessages.value.clear()
}

const exportSelectedMessages = async () => {
  const selectedMessagesList = messages.value.filter(msg => 
    selectedMessages.value.has(msg.id) && !msg.isPreview
  )
  
  if (selectedMessagesList.length === 0) return
  
  try {
    // For now, just log the messages that would be exported
    console.log('Exporting messages:', selectedMessagesList)
    showExportControls.value = false
    selectedMessages.value.clear()
  } catch (error) {
    console.error('Failed to export messages:', error)
  }
}

// Sidebar actions
const toggleConversationSidebar = async () => {
  showConversationSidebar.value = !showConversationSidebar.value
  if (showConversationSidebar.value) {
    console.log('📁 ConversationalWindow: Loading conversations for sidebar')
    console.log('📁 ConversationalWindow: Store sessions before loading:', conversationStore.sessions.length)
    await loadConversations()
    console.log('📁 ConversationalWindow: Store sessions after loading:', conversationStore.sessions.length)
    console.log('📁 ConversationalWindow: After loading, allConversations:', allConversations.value.length)
  }
}

const handleNewConversation = async () => {
  await createNewConversation()
  console.log('📁 ConversationalWindow: After creating new conversation, allConversations:', allConversations.value.length)
  showConversationSidebar.value = false
}

const handleResumeConversation = async (id: string) => {
  await resumeConversation(id)
  showConversationSidebar.value = false
}

const handleDeleteConversation = async (id: string) => {
  await deleteConversation(id)
}

// AI Assistant actions
const toggleAIAssistant = () => {
  if (showAIAssistant.value) {
    // If already open, just close it
    showAIAssistant.value = false
  } else {
    // Close other drawers first, then open AI Assistant
    showLiveAI.value = false
    showAIAssistant.value = true
  }
}

const handleAIQuery = async (query: string) => {
  const currentMessages = conversationStore.currentMessages || []
  await queryAI(query, currentMessages)
}

// Live AI actions
const toggleLiveAI = () => {
  if (showLiveAI.value) {
    // If already open, just close it
    showLiveAI.value = false
  } else {
    // Close other drawers first, then open Live AI
    showAIAssistant.value = false
    showLiveAI.value = true
  }
}

const toggleLiveAIActive = async () => {
  if (isLiveAIActive.value) {
    await stopLiveAI()
  } else {
    const currentMessages = conversationStore.currentMessages || []
    await startLiveAI(currentMessages)
  }
}

// Instant-response window close with background cleanup
const closeWindow = async () => {
  try {
    console.log('🚪 INSTANT CLOSE - Window closing immediately with background cleanup')
    
    // IMMEDIATE WINDOW CLOSE - Don't wait for cleanup
    emit('close')
    emit('update:showConversationalWindow', false)
    console.log('✅ Window closed instantly for user')
    
    // Background cleanup after window is closed
    if (isRecording.value || isAudioLoopbackActive.value) {
      console.log('🧹 Starting background cleanup after window close...')
      
      // Send instant cancellation signals
      try {
        await Promise.all([
          invoke('cancel_all_transcriptions').catch(console.warn),
          invoke('emergency_shutdown_whisper').catch(console.warn)
        ])
        console.log('✅ Emergency shutdown signals sent')
      } catch (signalError) {
        console.warn('⚠️ Emergency signals failed:', signalError)
      }
      
      // Background cleanup with aggressive timeout
      const backgroundCleanup = async () => {
        const cleanupPromises = []
        
        if (isRecording.value) {
          cleanupPromises.push(
            stopRecording().catch(err => {
              console.error('❌ Background microphone cleanup failed:', err)
              // Force reset state
              isRecording.value = false
            })
          )
        }
        
        if (isAudioLoopbackActive.value) {
          cleanupPromises.push(
            stopAudioLoopbackCapture().catch(err => {
              console.error('❌ Background loopback cleanup failed:', err)
              // Force reset state
              conversationStore.setAudioLoopbackState(false)
            })
          )
        }
        
        // Aggressive cleanup with short timeout
        try {
          await Promise.race([
            Promise.allSettled(cleanupPromises),
            new Promise(resolve => setTimeout(resolve, 1000)) // 1 second max
          ])
          console.log('✅ Background audio cleanup completed')
        } catch (error) {
          console.warn('⚠️ Background audio cleanup timeout:', error)
        }
        
        // Force cleanup all whisper contexts
        try {
          await Promise.race([
            invoke('force_cleanup_whisper_contexts'),
            new Promise(resolve => setTimeout(resolve, 500)) // 500ms max
          ])
          console.log('✅ Background whisper cleanup completed')
        } catch (whisperError) {
          console.warn('⚠️ Background whisper cleanup failed:', whisperError)
        }
        
        // Final state reset
        isRecording.value = false
        conversationStore.setAudioLoopbackState(false)
        console.log('✅ Background cleanup sequence completed')
      }
      
      // Execute background cleanup without blocking
      backgroundCleanup()
    }
    
  } catch (error) {
    console.error('❌ Error during instant window close:', error)
    
    // Ensure window still closes even if cleanup fails
    emit('close')
    emit('update:showConversationalWindow', false)
    
    // Emergency state reset
    isRecording.value = false
    conversationStore.setAudioLoopbackState(false)
  }
}

// UI helper functions
const formatSessionDuration = () => {
  if (!conversationStore.currentSession) return '00:00'
  
  const startTime = conversationStore.currentSession.startTime
  const currentTime = Date.now()
  const duration = Math.floor((currentTime - startTime) / 1000)
  
  const minutes = Math.floor(duration / 60)
  const seconds = duration % 60
  
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
}


</script>

<template>
  <Transition name="conversational-window">
    <div v-if="showConversationalWindow" ref="conversationalWindowRef" class="conversational-window">
      <!-- Window Header -->
      <div class="window-header">
        <div class="header-title">
          <div class="flex items-center gap-2">
            <MicrophoneIcon class="w-4 h-4 text-white/80" />
            <span class="text-sm font-medium text-white/90">Conversation</span>
          </div>
          <div class="header-controls">
            <div class="status-indicators">
              <button 
                @click="toggleExportControls" 
                class="export-btn"
                :class="{ 'active': showExportControls }"
                title="Export conversation"
              >
                <PencilIcon class="w-3 h-3" />
              </button>
              <button 
                @click="toggleConversationSidebar" 
                class="export-btn"
                :class="{ 'active': showConversationSidebar }"
                title="Show conversations"
              >
                <QueueListIcon class="w-3 h-3" />
              </button>
              <button 
                @click="toggleAIAssistant" 
                class="ai-assistant-btn"
                :class="{ 'active': showAIAssistant }"
                title="AI Assistant"
              >
                <SparklesIcon class="w-3 h-3" />
              </button>
              <button 
                @click="toggleLiveAI" 
                class="live-ai-btn"
                :class="{ 'active': showLiveAI }"
                title="Live AI Response"
              >
                <RocketLaunchIcon class="w-3 h-3" />
              </button>
              <div v-if="isRecording" class="status-indicator" title="Recording">
                <div class="recording-dot"></div>
              </div>
              <div v-if="isAudioLoopbackActive" class="status-indicator" title="Audio loopback active">
                <SpeakerWaveIcon class="w-3 h-3 text-purple-400 animate-pulse" />
              </div>
            </div>
            <button @click="closeWindow" class="close-btn">
              <XMarkIcon class="w-5 h-5 text-white/70 hover:text-white" />
            </button>
          </div>
        </div>
      </div>
      
      <!-- Window Content Container -->
      <div class="window-content">
        <!-- Conversation Sidebar -->
        <ConversationSidebarAdapter
          :show="showConversationSidebar"
          :conversations="allConversations"
          :is-loading="isLoadingConversations"
          @close="showConversationSidebar = false"
          @new-conversation="handleNewConversation"
          @resume-conversation="handleResumeConversation"
          @delete-conversation="handleDeleteConversation"
        />
        
        <!-- Main Content Area -->
        <div class="main-content" :class="{ 'with-sidebar': showConversationSidebar }">
          <!-- Export Controls -->
          <ExportControls
            :show="showExportControls"
            :selected-count="selectedMessages.size"
            :has-selection="hasSelectedMessages"
            @close="toggleExportControls"
            @export="exportSelectedMessages"
            @select-all="selectAllMessages"
            @deselect-all="deselectAllMessages"
          />
          
          <!-- Conversation Area -->
          <div class="conversation-area">
            <MessageList
              ref="messageListRef"
              :messages="messages"
              :show-export-controls="showExportControls"
              :selected-messages="selectedMessages"
              @toggle-message-selection="toggleMessageSelection"
            />
          </div>
          
          <!-- Bottom Action Bar -->
          <div class="action-bar">
            <!-- Compact Single Row Layout -->
            <div class="action-row">
              <!-- Status Indicators -->
              <div class="status-indicators">
                <div class="status-item" :class="{ 'active': isRecording }">
                  <MicrophoneIcon class="w-3 h-3" />
                  <span class="status-label">Mic</span>
                  <div class="status-dot" :class="{ 'active': isRecording }"></div>
                </div>
                <div class="status-item" :class="{ 'active': isAudioLoopbackActive }">
                  <SpeakerWaveIcon class="w-3 h-3" />
                  <span class="status-label">Audio</span>
                  <div class="status-dot" :class="{ 'active': isAudioLoopbackActive }"></div>
                </div>
                <div v-if="conversationStore.currentSession" class="status-item time-item">
                  <span class="time-label">{{ formatSessionDuration() }}</span>
                </div>
              </div>

              <!-- Compact Action Button -->
              <button 
                @click="toggleMicrophone" 
                class="compact-action-btn"
                :class="{ 'active': isRecording, 'disabled': !isSpeechInitialized }"
                :disabled="!isSpeechInitialized"
                :title="isRecording ? 'Stop Recording' : 'Start Recording'"
              >
                <MicrophoneIcon class="w-4 h-4" />
                <span class="btn-label">{{ isRecording ? 'Stop' : 'Start' }}</span>
              </button>

              <!-- Saving Indicator -->
              <div v-if="isSaving" class="saving-indicator">
                <div class="saving-spinner"></div>
                <span class="saving-text">Saving...</span>
              </div>

              <!-- Error Display -->
              <div v-if="speechError" class="compact-error">
                <XMarkIcon class="w-3 h-3 text-red-400" />
                <span class="error-text">{{ speechError }}</span>
              </div>
            </div>
          </div>
        </div>
        
        <!-- AI Assistant Drawer -->
        <AIAssistant
          :show="showAIAssistant"
          :processing="aiIsProcessing"
          :response="aiResponse"
          :error="aiError"
          :message-count="messages.length"
          @close="showAIAssistant = false"
          @query="handleAIQuery"
        />
        
        <!-- Live AI Drawer -->
        <LiveAI
          :show="showLiveAI"
          :is-active="isLiveAIActive"
          :processing="liveAIIsProcessing"
          :response="liveAIResponse"
          :suggestions="liveAISuggestions"
          :error="liveAIError"
          @close="showLiveAI = false"
          @toggle-live="toggleLiveAIActive"
        />
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.conversational-window {
  @apply backdrop-blur-xl border border-white/15 rounded-2xl overflow-hidden;
  background: linear-gradient(to bottom, 
    rgba(10, 10, 12, 0.9) 0%, 
    rgba(5, 5, 7, 0.95) 100%
  );
  width: 600px;
  height: 700px;
  max-width: 95vw;
  max-height: calc(100vh - 100px); /* Account for control panel */
  display: flex;
  flex-direction: column;
  position: relative;
  
  /* Enhanced glass effect similar to transparency controls */
  backdrop-filter: blur(80px) saturate(180%);
  box-shadow: 
    0 0 0 1px rgba(255, 255, 255, 0.1),
    0 10px 30px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

/* Dynamic width based on visible panels */
.conversational-window:has(.conversation-sidebar) {
  width: 980px;
  max-width: 95vw;
}

.conversational-window:has(.ai-assistant-drawer) {
  width: 980px;
  max-width: 95vw;
}

.conversational-window:has(.live-ai-drawer) {
  width: 980px;
  max-width: 95vw;
}

.conversational-window:has(.conversation-sidebar):has(.ai-assistant-drawer) {
  width: 1280px;
  max-width: 95vw;
}

.conversational-window:has(.conversation-sidebar):has(.live-ai-drawer) {
  width: 1280px;
  max-width: 95vw;
}

.conversational-window:has(.ai-assistant-drawer):has(.live-ai-drawer) {
  width: 1280px;
  max-width: 95vw;
}

.conversational-window:has(.conversation-sidebar):has(.ai-assistant-drawer):has(.live-ai-drawer) {
  width: 1600px;
  max-width: 95vw;
}

.window-content {
  @apply flex-1 flex flex-col min-h-0;
}

.conversational-window:has(.conversation-sidebar) .window-content,
.conversational-window:has(.ai-assistant-drawer) .window-content,
.conversational-window:has(.live-ai-drawer) .window-content {
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

.status-indicators {
  @apply flex items-center gap-2;
}

.export-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-all duration-200 text-white/60 hover:text-white/90;
}

.export-btn.active {
  @apply bg-blue-500/20 text-blue-400 hover:bg-blue-500/30;
}

.ai-assistant-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-all duration-200 text-white/60 hover:text-white/90;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(147, 51, 234, 0.1));
  border: 1px solid rgba(59, 130, 246, 0.2);
}

.ai-assistant-btn.active {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.4), rgba(147, 51, 234, 0.4));
  border-color: rgba(59, 130, 246, 0.6);
}

.live-ai-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-all duration-200 text-white/60 hover:text-white/90;
  background: linear-gradient(135deg, rgba(251, 146, 60, 0.1), rgba(234, 88, 12, 0.1));
  border: 1px solid rgba(251, 146, 60, 0.2);
}

.live-ai-btn.active {
  background: linear-gradient(135deg, rgba(251, 146, 60, 0.4), rgba(234, 88, 12, 0.4));
  border-color: rgba(251, 146, 60, 0.6);
}

.status-indicator {
  @apply flex items-center justify-center w-6 h-6 rounded-full bg-white/10;
}

.recording-dot {
  @apply w-2 h-2 bg-red-500 rounded-full animate-pulse;
}

.close-btn {
  @apply rounded-full p-1 hover:bg-white/10 transition-colors;
}

.conversation-area {
  @apply flex-1 overflow-y-auto;
  min-height: 0;
}

/* Scrollbar styles */
.conversation-area::-webkit-scrollbar {
  width: 4px;
}

.conversation-area::-webkit-scrollbar-track {
  background: transparent;
}

.conversation-area::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

.conversation-area::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

.action-bar {
  @apply px-4 py-3 border-t border-white/10;
  background: linear-gradient(to top, 
    rgba(0, 0, 0, 0.4) 0%, 
    rgba(0, 0, 0, 0.2) 100%
  );
  backdrop-filter: blur(10px);
}

/* Compact Action Row */
.action-row {
  @apply flex items-center justify-between gap-4;
}

/* Status Indicators */
.status-indicators {
  @apply flex items-center gap-3;
}

.status-item {
  @apply flex items-center gap-1.5 px-2 py-1.5 rounded-md transition-all duration-200;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.status-item.active {
  background: rgba(34, 197, 94, 0.1);
  border-color: rgba(34, 197, 94, 0.3);
}

.status-item.time-item {
  @apply px-2 py-1;
  background: rgba(59, 130, 246, 0.1);
  border-color: rgba(59, 130, 246, 0.2);
}

.status-label {
  @apply text-xs font-medium text-white/70;
}

.status-item.active .status-label {
  @apply text-green-400;
}

.status-dot {
  @apply w-1.5 h-1.5 rounded-full bg-white/30 transition-all duration-200;
}

.status-dot.active {
  @apply bg-green-400 animate-pulse;
}

.time-label {
  @apply text-xs text-blue-300 font-mono;
}

/* Compact Action Button */
.compact-action-btn {
  @apply flex items-center gap-2 px-3 py-2 rounded-lg transition-all duration-200;
  background: linear-gradient(135deg, 
    rgba(59, 130, 246, 0.2) 0%, 
    rgba(147, 51, 234, 0.2) 100%
  );
  border: 1px solid rgba(59, 130, 246, 0.3);
  backdrop-filter: blur(10px);
}

.compact-action-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, 
    rgba(59, 130, 246, 0.3) 0%, 
    rgba(147, 51, 234, 0.3) 100%
  );
  border-color: rgba(59, 130, 246, 0.5);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.2);
}

.compact-action-btn.active {
  background: linear-gradient(135deg, 
    rgba(239, 68, 68, 0.6) 0%, 
    rgba(220, 38, 38, 0.6) 100%
  );
  border-color: rgba(239, 68, 68, 0.5);
  box-shadow: 0 2px 8px rgba(239, 68, 68, 0.3);
}

.compact-action-btn.active:hover {
  background: linear-gradient(135deg, 
    rgba(239, 68, 68, 0.7) 0%, 
    rgba(220, 38, 38, 0.7) 100%
  );
  border-color: rgba(239, 68, 68, 0.6);
}

.compact-action-btn:disabled {
  @apply opacity-50 cursor-not-allowed;
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.1);
}

.btn-label {
  @apply text-xs font-medium text-white;
}

/* Compact Error Display */
.compact-error {
  @apply flex items-center gap-1.5 px-2 py-1.5 rounded-md;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
}

.error-text {
  @apply text-xs text-red-300;
}

/* Saving Indicator */
.saving-indicator {
  @apply flex items-center gap-1.5 px-2 py-1.5 rounded-md;
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.3);
}

.saving-spinner {
  @apply w-3 h-3 border-2 border-blue-400 border-t-transparent rounded-full;
  animation: spin 1s linear infinite;
}

.saving-text {
  @apply text-xs text-blue-300 font-medium;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Transition styles */
.conversational-window-enter-active,
.conversational-window-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.conversational-window-enter-from {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}

.conversational-window-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}
</style>