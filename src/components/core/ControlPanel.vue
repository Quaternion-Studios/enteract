<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useAppStore } from '../../stores/app'
import { useMLEyeTracking } from '../../composables/useMLEyeTracking'
import { useWindowManager } from '../../composables/useWindowManager'
import { useWindowResizing } from '../../composables/useWindowResizing'
import { useAIModels } from '../../composables/useAIModels'
import { useControlPanelState } from '../../composables/useControlPanelState'
import { useControlPanelEvents } from '../../composables/useControlPanelEvents'
import ControlPanelButtons from './ControlPanelButtons.vue'
import PanelWindows from './PanelWindows.vue'

interface Emits {
  (e: 'toggle-chat-drawer'): void
}

const emit = defineEmits<Emits>()

const store = useAppStore()
const mlEyeTracking = useMLEyeTracking()
const windowManager = useWindowManager()
const { resizeWindow } = useWindowResizing()
const { selectedModel } = useAIModels()

// State management
const {
  isDragging,
  dragStartTime,
  showChatWindow,
  showAIModelsWindow,
  showConversationalWindow,
  speechError,
  compatibilityReport,
  isGazeControlActive,
  dragIndicatorVisible,
  // Window management functions
  closeAllWindows,
  openWindow,
  toggleWindow,
  getWindowState,
  hasOpenWindow
} = useControlPanelState()

// Event handlers
const {
  handleDragStart,
  handleDragEnd,
  toggleAIModelsWindow,
  closeAIModelsWindow,
  toggleChatWindow,
  closeChatWindow,
  openChatWindow,
  toggleConversationalWindow,
  closeConversationalWindow,
  toggleMLEyeTrackingWithMovement,
  handleKeydown
} = useControlPanelEvents(
  store,
  mlEyeTracking,
  windowManager,
  null, // Remove wake word detection
  {
    isDragging,
    dragStartTime,
    showChatWindow,
    showAIModelsWindow,
    showConversationalWindow,
    speechError,
    compatibilityReport,
    isGazeControlActive,
    dragIndicatorVisible,
    // Window management functions
    closeAllWindows,
    openWindow,
    toggleWindow,
    getWindowState,
    hasOpenWindow
  }
)

// Watch for window state changes to resize window
watch(showChatWindow, async (newValue) => {
  await resizeWindow(newValue, false, showAIModelsWindow.value, showConversationalWindow.value, false)
})

watch(showAIModelsWindow, async (newValue) => {
  await resizeWindow(showChatWindow.value, false, newValue, showConversationalWindow.value, false)
})

watch(showConversationalWindow, async (newValue) => {
  console.log(`🔧 CONVERSATIONAL WATCH: newValue=${newValue}`)
  await resizeWindow(showChatWindow.value, false, showAIModelsWindow.value, newValue, false)
})

// Window update handlers that enforce exclusivity using centralized window manager
const handleSettingsPanelUpdate = async (value: boolean) => {
  if (value) {
    await openWindow('aiModels')
  } else {
    await closeAllWindows()
  }
}

const handleChatWindowUpdate = async (value: boolean) => {
  if (value) {
    await openWindow('chat')
  } else {
    await closeAllWindows()
  }
}

const handleConversationalWindowUpdate = async (value: boolean) => {
  if (value) {
    await openWindow('conversational')
  } else {
    await closeAllWindows()
  }
}

// Expose the openChatWindow method for parent components
defineExpose({
  openChatWindow
})

// Ref for the control panel element
const controlPanelRef = ref<HTMLElement>()

onMounted(async () => {
  document.addEventListener('keydown', handleKeydown)
  // Removed drag event listeners - using Tauri's native drag region

  await store.initializeSpeechTranscription('tiny')

  await resizeWindow(false, false, false, false, false)

  console.log('⌨️ Keyboard Shortcuts:')
  console.log('   Ctrl+Shift+E = Start/Stop ML Eye Tracking + Window Movement')
  console.log('   Ctrl+Shift+S = Emergency Stop (stop all tracking)')
  console.log('   Ctrl+Shift+C = Toggle Chat Window')
  console.log('   Ctrl+Shift+A = Toggle AI Models Window')
  console.log('   Escape = Close any open panels')
  console.log('🎯 Control Panel is draggable - use the dots on the left to drag!')
  console.log('📐 Chat Window is resizable - drag the resize handles!')
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="app-layout">
    <!-- Control Panel Section -->
    <div class="control-panel-section">
      <div
        ref="controlPanelRef"
        class="control-panel-glass-bar"
        :class="{ 'dragging': isDragging }"
      >
        <!-- Dedicated drag handle area on the left -->
        <div class="drag-handle" data-tauri-drag-region>
          <div class="drag-dots">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </div>

        <ControlPanelButtons
          :store="store"
          :mlEyeTracking="mlEyeTracking"
          :showChatWindow="showChatWindow"
          :showAIModelsWindow="showAIModelsWindow"
          :showConversationalWindow="showConversationalWindow"
          :isGazeControlActive="isGazeControlActive"
          @toggle-ai-models="toggleAIModelsWindow"
          @toggle-eye-tracking="toggleMLEyeTrackingWithMovement"
          @toggle-conversational="toggleConversationalWindow"
          @toggle-chat="toggleChatWindow"
        />

        <!-- Spacer to balance layout -->
        <div class="drag-spacer"></div>
      </div>
    </div>

    <!-- Panel Windows Container -->
    <div class="panel-windows-container">
      <PanelWindows
        :showSettingsPanel="showAIModelsWindow"
        :showChatWindow="showChatWindow"
        :showConversationalWindow="showConversationalWindow"
        :selectedModel="selectedModel"
        @close-settings="closeAIModelsWindow"
        @close-chat="closeChatWindow"
        @close-conversational="closeConversationalWindow"
        @update:show-settings-panel="handleSettingsPanelUpdate($event)"
        @update:show-chat-window="handleChatWindowUpdate($event)"
        @update:show-conversational-window="handleConversationalWindowUpdate($event)"
        @update:selected-model="selectedModel = $event"
        @toggle-chat-drawer="emit('toggle-chat-drawer')"
      />
    </div>
  </div>
</template>

<style scoped>
.app-layout {
  @apply w-full h-full bg-transparent;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
}

.control-panel-section {
  @apply w-full flex justify-center;
  height: 60px;
  padding: 8px;
  background: transparent;
  position: fixed;
  top: 0;
  z-index: 100;
}

/* Curved Glass Control Panel Bar */
.control-panel-glass-bar {
  @apply rounded-full overflow-hidden relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  width: 320px;
  user-select: none;

  /* Premium curved glass effect with darker background */
  background: linear-gradient(135deg,
    rgba(10, 10, 12, 0.90) 0%,
    rgba(10, 10, 12, 0.80) 25%,
    rgba(10, 10, 12, 0.75) 50%,
    rgba(10, 10, 12, 0.80) 75%,
    rgba(10, 10, 12, 0.90) 100%
  );
  backdrop-filter: blur(40px) saturate(180%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.3);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.25),
    0 2px 8px rgba(0, 0, 0, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1);

  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.control-panel-glass-bar:hover {
  background: linear-gradient(135deg, 
    rgba(10, 10, 12, 0.95) 0%,
    rgba(10, 10, 12, 0.85) 25%,
    rgba(10, 10, 12, 0.80) 50%,
    rgba(10, 10, 12, 0.85) 75%,
    rgba(10, 10, 12, 0.95) 100%
  );
  border-color: rgba(255, 255, 255, 0.4);
  box-shadow: 
    0 12px 40px rgba(0, 0, 0, 0.3),
    0 4px 12px rgba(0, 0, 0, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.5),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1);
  transform: translateY(-1px);
}

/* Dragging state */
.control-panel-glass-bar.dragging {
  cursor: grabbing;
  transform: translateY(-2px) scale(1.02);
  box-shadow: 
    0 16px 50px rgba(0, 0, 0, 0.4),
    0 8px 20px rgba(0, 0, 0, 0.3),
    inset 0 1px 0 rgba(255, 255, 255, 0.6),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1);
  border-color: rgba(255, 255, 255, 0.5);
  background: linear-gradient(135deg, 
    rgba(10, 10, 12, 1) 0%,
    rgba(10, 10, 12, 0.90) 25%,
    rgba(10, 10, 12, 0.85) 50%,
    rgba(10, 10, 12, 0.90) 75%,
    rgba(10, 10, 12, 1) 100%
  );
}

/* Drag handle - visible drag area */
.drag-handle {
  @apply flex items-center justify-center;
  width: 40px;
  height: 100%;
  cursor: grab;
  flex-shrink: 0;
  -webkit-app-region: drag;
}

.drag-handle:active {
  cursor: grabbing;
}

.drag-dots {
  @apply flex flex-col gap-1;
  opacity: 0.6;
  transition: opacity 0.2s ease;
}

.drag-handle:hover .drag-dots {
  opacity: 1;
}

.drag-dots span {
  @apply w-1 h-1 rounded-full bg-white/60;
  display: block;
}

/* Spacer to balance layout */
.drag-spacer {
  width: 40px;
  height: 100%;
  flex-shrink: 0;
}

/* Floating animation for the entire bar (disabled when dragging) */
.control-panel-glass-bar:not(.dragging) {
  animation: float 6s ease-in-out infinite;
}

@keyframes float {
  0%, 100% {
    transform: translateY(0px);
  }
  50% {
    transform: translateY(-2px);
  }
}

.control-panel-glass-bar:hover:not(.dragging) {
  animation: none;
}

/* Panel Windows Container */
.panel-windows-container {
  @apply w-full flex-1;
  margin-top: 68px; /* Account for fixed control panel height + padding */
  padding: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
}

/* Removed - drag region now on .drag-handle */
</style>