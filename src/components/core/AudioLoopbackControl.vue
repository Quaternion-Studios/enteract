<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { MicrophoneIcon, SpeakerWaveIcon, StopIcon } from '@heroicons/vue/24/outline'
import { useAudioLoopback } from '../../composables/useAudioLoopback'

interface Props {
  enabled?: boolean
}

interface Emits {
  (e: 'transcription', text: string): void
  (e: 'update:enabled', value: boolean): void
}

const props = withDefaults(defineProps<Props>(), {
  enabled: false
})

const emit = defineEmits<Emits>()

const {
  selectedDevice,
  isCapturing,
  audioLevel,
  captureError,
  transcriptions,
  isProcessingAudio,
  settings,
  canCapture,
  startCapture,
  stopCapture,
  loadSettings
} = useAudioLoopback()

// Local state
const showTranscriptions = ref(false)

// Computed
const audioLevelPercent = computed(() => {
  // Convert dB to percentage (0-100)
  // -60dB = 0%, 0dB = 100%
  const percent = ((audioLevel.value + 60) / 60) * 100
  return Math.max(0, Math.min(100, percent))
})

const statusText = computed(() => {
  if (isCapturing.value) {
    return isProcessingAudio.value ? 'Processing...' : 'Listening...'
  }
  if (captureError.value) {
    return 'Error'
  }
  if (selectedDevice.value) {
    return 'Ready'
  }
  return 'No device'
})

const statusColor = computed(() => {
  if (captureError.value) return 'text-red-400'
  if (isCapturing.value) return 'text-green-400'
  if (selectedDevice.value) return 'text-yellow-400'
  return 'text-gray-400'
})

// Watch for new transcriptions
watch(transcriptions, (newTranscriptions) => {
  if (newTranscriptions.length > 0) {
    const latest = newTranscriptions[newTranscriptions.length - 1]
    emit('transcription', latest.text)
  }
}, { deep: true })

// Watch enabled prop
watch(() => props.enabled, async (enabled) => {
  if (enabled && canCapture.value && !isCapturing.value) {
    await startCapture()
  } else if (!enabled && isCapturing.value) {
    await stopCapture()
  }
})

// Toggle capture
const toggleCapture = async () => {
  if (isCapturing.value) {
    await stopCapture()
    emit('update:enabled', false)
  } else if (canCapture.value) {
    await startCapture()
    emit('update:enabled', true)
  }
}

// Load settings on mount
onMounted(async () => {
  await loadSettings()
  
  // Auto-start if enabled in settings
  if (settings.value.loopbackEnabled && props.enabled) {
    await startCapture()
  }
})
</script>

<template>
  <div class="audio-loopback-control">
    <!-- Main Control Button -->
    <div class="control-section">
      <button
        @click="toggleCapture"
        :disabled="!selectedDevice"
        :class="[
          'capture-button',
          isCapturing ? 'capturing' : '',
          captureError ? 'error' : ''
        ]"
        :title="isCapturing ? 'Stop Audio Capture' : 'Start Audio Capture'"
      >
        <StopIcon v-if="isCapturing" class="w-5 h-5" />
        <MicrophoneIcon v-else class="w-5 h-5" />
      </button>
      
      <!-- Audio Level Indicator -->
      <div class="audio-level-container">
        <div class="audio-level-background">
          <div 
            class="audio-level-bar"
            :style="{ width: audioLevelPercent + '%' }"
            :class="{ active: isCapturing }"
          />
        </div>
        <span class="status-text" :class="statusColor">
          {{ statusText }}
        </span>
      </div>
      
      <!-- Device Info -->
      <div v-if="selectedDevice" class="device-info">
        <SpeakerWaveIcon class="w-4 h-4 text-white/60" />
        <span class="device-name">{{ selectedDevice.name }}</span>
      </div>
    </div>
    
    <!-- Error Display -->
    <div v-if="captureError" class="error-message">
      <span class="text-red-400 text-xs">{{ captureError }}</span>
    </div>
    
    <!-- Transcription Indicator -->
    <div v-if="isCapturing && transcriptions.length > 0" class="transcription-indicator">
      <button
        @click="showTranscriptions = !showTranscriptions"
        class="transcription-toggle"
      >
        <span class="dot" :class="{ processing: isProcessingAudio }"></span>
        <span class="text">{{ transcriptions.length }} transcriptions</span>
      </button>
    </div>
    
    <!-- Recent Transcriptions (collapsible) -->
    <Transition name="transcriptions">
      <div v-if="showTranscriptions && transcriptions.length > 0" class="recent-transcriptions">
        <div class="transcriptions-header">
          <span class="text-white/80 text-xs font-medium">Recent Transcriptions</span>
          <button
            @click="showTranscriptions = false"
            class="close-btn"
          >
            ×
          </button>
        </div>
        <div class="transcriptions-list">
          <div
            v-for="(transcription, index) in transcriptions.slice(-5).reverse()"
            :key="index"
            class="transcription-item"
          >
            <span class="transcription-text">{{ transcription.text }}</span>
            <span class="transcription-time">
              {{ new Date(transcription.timestamp).toLocaleTimeString() }}
            </span>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.audio-loopback-control {
  @apply space-y-2;
}

.control-section {
  @apply flex items-center gap-3;
}

.capture-button {
  @apply p-3 rounded-xl bg-white/10 hover:bg-white/20 transition-all duration-200;
  @apply text-white/70 hover:text-white;
  @apply border border-white/20;
  @apply disabled:opacity-50 disabled:cursor-not-allowed;
}

.capture-button.capturing {
  @apply bg-green-500/20 hover:bg-green-500/30;
  @apply text-green-400 border-green-400/50;
  @apply animate-pulse;
}

.capture-button.error {
  @apply bg-red-500/20 hover:bg-red-500/30;
  @apply text-red-400 border-red-400/50;
}

.audio-level-container {
  @apply flex-1 relative;
}

.audio-level-background {
  @apply h-8 bg-white/5 rounded-lg overflow-hidden;
  @apply border border-white/10;
}

.audio-level-bar {
  @apply h-full bg-gradient-to-r from-green-500/50 to-green-400/50;
  @apply transition-all duration-100 ease-out;
}

.audio-level-bar.active {
  @apply from-green-500 to-green-400;
}

.status-text {
  @apply absolute inset-0 flex items-center justify-center;
  @apply text-xs font-medium;
}

.device-info {
  @apply flex items-center gap-2;
  @apply px-3 py-1.5 bg-white/5 rounded-lg;
  @apply border border-white/10;
}

.device-name {
  @apply text-white/70 text-xs truncate max-w-[150px];
}

.error-message {
  @apply px-3 py-2 bg-red-500/10 border border-red-400/30 rounded-lg;
}

.transcription-indicator {
  @apply mt-2;
}

.transcription-toggle {
  @apply flex items-center gap-2 px-3 py-1.5;
  @apply bg-white/5 hover:bg-white/10 rounded-lg;
  @apply border border-white/10 hover:border-white/20;
  @apply transition-all duration-200;
  @apply text-white/70 hover:text-white text-xs;
}

.transcription-toggle .dot {
  @apply w-2 h-2 bg-green-400 rounded-full;
}

.transcription-toggle .dot.processing {
  @apply animate-pulse bg-yellow-400;
}

.recent-transcriptions {
  @apply mt-2 p-3 bg-white/5 rounded-lg border border-white/10;
}

.transcriptions-header {
  @apply flex items-center justify-between mb-2;
}

.close-btn {
  @apply text-white/60 hover:text-white text-lg leading-none;
  @apply transition-colors;
}

.transcriptions-list {
  @apply space-y-1 max-h-32 overflow-y-auto;
}

.transcription-item {
  @apply flex items-start justify-between gap-2;
  @apply p-2 bg-white/5 rounded;
  @apply text-xs;
}

.transcription-text {
  @apply text-white/80 flex-1;
}

.transcription-time {
  @apply text-white/40 text-[10px] whitespace-nowrap;
}

/* Transitions */
.transcriptions-enter-active,
.transcriptions-leave-active {
  transition: all 0.2s ease;
}

.transcriptions-enter-from,
.transcriptions-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

/* Scrollbar */
.transcriptions-list::-webkit-scrollbar {
  width: 4px;
}

.transcriptions-list::-webkit-scrollbar-track {
  background: transparent;
}

.transcriptions-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}</style>