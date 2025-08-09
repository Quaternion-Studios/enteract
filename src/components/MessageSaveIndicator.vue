<template>
  <div class="message-save-indicator">
    <!-- Individual message save status -->
    <div 
      v-if="message?.persistenceState"
      class="save-status"
      :class="`status-${message.persistenceState}`"
      :title="getStatusTooltip()"
    >
      <span v-if="message.persistenceState === 'pending'" class="status-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" stroke-dasharray="2 2" class="rotating"/>
        </svg>
      </span>
      
      <span v-else-if="message.persistenceState === 'saving'" class="status-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="spin">
          <path d="M12 2v4m0 12v4m-8-8h4m12 0h4"/>
        </svg>
      </span>
      
      <span v-else-if="message.persistenceState === 'saved'" class="status-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
      </span>
      
      <span v-else-if="message.persistenceState === 'failed'" class="status-icon">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <span v-if="message.retryCount" class="retry-count">{{ message.retryCount }}</span>
      </span>
    </div>
    
    <!-- Global save queue status -->
    <div v-if="showGlobalStatus" class="global-status">
      <div v-if="queueStatus.pendingCount > 0" class="queue-info">
        <span class="queue-icon">📤</span>
        <span class="queue-count">{{ queueStatus.pendingCount }} pending</span>
      </div>
      
      <div v-if="queueStatus.failedCount > 0" class="queue-info error">
        <span class="queue-icon">⚠️</span>
        <span class="queue-count">{{ queueStatus.failedCount }} failed</span>
      </div>
      
      <div v-if="!queueStatus.isOnline" class="queue-info offline">
        <span class="queue-icon">📴</span>
        <span class="queue-text">Offline - Messages queued</span>
      </div>
      
      <div v-if="queueStatus.isSaving" class="queue-info saving">
        <span class="queue-icon spin">💾</span>
        <span class="queue-text">Saving...</span>
      </div>
    </div>
    
    <!-- Save statistics (debug/admin view) -->
    <div v-if="showStats && queueStatus.stats" class="save-stats">
      <div class="stat-item">
        <span class="stat-label">Saved:</span>
        <span class="stat-value">{{ queueStatus.stats.totalSaved }}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Failed:</span>
        <span class="stat-value error">{{ queueStatus.stats.totalFailed }}</span>
      </div>
      <div v-if="queueStatus.stats.averageSaveTime > 0" class="stat-item">
        <span class="stat-label">Avg time:</span>
        <span class="stat-value">{{ Math.round(queueStatus.stats.averageSaveTime) }}ms</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useConversationStore } from '../stores/conversation'
import type { ConversationMessage } from '../stores/conversation'

interface Props {
  message?: ConversationMessage
  showGlobalStatus?: boolean
  showStats?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  showGlobalStatus: false,
  showStats: false
})

const conversationStore = useConversationStore()

// Get queue status
const queueStatus = computed(() => conversationStore.getMessagePersistenceStatus())

// Get tooltip text based on status
const getStatusTooltip = () => {
  if (!props.message) return ''
  
  switch (props.message.persistenceState) {
    case 'pending':
      return 'Message queued for saving'
    case 'saving':
      return 'Saving message...'
    case 'saved':
      return 'Message saved successfully'
    case 'failed':
      if (props.message.retryCount && props.message.retryCount >= 3) {
        return `Save failed - Max retries exceeded: ${props.message.saveError || 'Unknown error'}`
      }
      return `Save failed - Retry ${props.message.retryCount || 0}/3: ${props.message.saveError || 'Unknown error'}`
    default:
      return ''
  }
}

// Auto-hide saved indicator after a delay
const showSavedIndicator = ref(true)
watch(() => props.message?.persistenceState, (newState) => {
  if (newState === 'saved') {
    showSavedIndicator.value = true
    setTimeout(() => {
      showSavedIndicator.value = false
    }, 3000) // Hide after 3 seconds
  } else {
    showSavedIndicator.value = true
  }
})
</script>

<style scoped>
.message-save-indicator {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}

.save-status {
  display: inline-flex;
  align-items: center;
  padding: 2px 4px;
  border-radius: 4px;
  transition: all 0.3s ease;
}

.status-icon {
  display: inline-flex;
  align-items: center;
  position: relative;
}

.status-pending {
  color: #666;
}

.status-saving {
  color: #3b82f6;
}

.status-saved {
  color: #10b981;
  animation: fadeIn 0.3s ease;
}

.status-saved:not(.show-saved) {
  opacity: 0;
  pointer-events: none;
}

.status-failed {
  color: #ef4444;
  animation: shake 0.5s ease;
}

.retry-count {
  position: absolute;
  top: -4px;
  right: -8px;
  background: #ef4444;
  color: white;
  border-radius: 50%;
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: bold;
}

.global-status {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 6px;
  font-size: 12px;
}

.queue-info {
  display: flex;
  align-items: center;
  gap: 4px;
}

.queue-info.error {
  color: #ef4444;
}

.queue-info.offline {
  color: #f59e0b;
}

.queue-info.saving {
  color: #3b82f6;
}

.save-stats {
  display: flex;
  gap: 12px;
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.02);
  border-radius: 4px;
  font-size: 10px;
  font-family: monospace;
}

.stat-item {
  display: flex;
  gap: 4px;
}

.stat-label {
  color: #666;
}

.stat-value {
  font-weight: bold;
}

.stat-value.error {
  color: #ef4444;
}

/* Animations */
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: scale(0.8);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes shake {
  0%, 100% {
    transform: translateX(0);
  }
  25% {
    transform: translateX(-2px);
  }
  75% {
    transform: translateX(2px);
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.spin {
  animation: spin 1s linear infinite;
}

.rotating {
  animation: spin 2s linear infinite;
}

/* Dark mode support */
@media (prefers-color-scheme: dark) {
  .global-status {
    background: rgba(255, 255, 255, 0.05);
  }
  
  .save-stats {
    background: rgba(255, 255, 255, 0.02);
  }
  
  .stat-label {
    color: #999;
  }
}
</style>