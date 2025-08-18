<template>
  <div class="database-status">
    <div class="status-card">
      <div class="status-header">
        <h3>📊 Database Status</h3>
        <button @click="refreshInfo" :disabled="loading" class="btn-refresh">
          {{ loading ? '⏳' : '🔄' }}
        </button>
      </div>

      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>Loading database information...</p>
      </div>

      <div v-else-if="dbInfo" class="info-grid">
        <div class="info-item">
          <div class="info-label">Status</div>
          <div class="info-value" :class="{ 'status-good': dbInfo.is_initialized, 'status-warning': !dbInfo.is_initialized }">
            {{ dbInfo.is_initialized ? '✅ Ready' : '⚠️ Not Initialized' }}
          </div>
        </div>

        <div class="info-item">
          <div class="info-label">Chat Sessions</div>
          <div class="info-value">{{ dbInfo.chat_sessions_count.toLocaleString() }}</div>
        </div>

        <div class="info-item">
          <div class="info-label">Conversations</div>
          <div class="info-value">{{ dbInfo.conversation_sessions_count.toLocaleString() }}</div>
        </div>

        <div class="info-item">
          <div class="info-label">Database Size</div>
          <div class="info-value">{{ dbInfo.database_size_mb.toFixed(2) }} MB</div>
        </div>
      </div>

      <div v-if="!dbInfo?.is_initialized" class="actions">
        <button 
          @click="initializeDatabase" 
          :disabled="initializing"
          class="btn-primary"
        >
          {{ initializing ? 'Initializing...' : '🚀 Initialize Database' }}
        </button>
      </div>

      <div v-if="dbInfo?.is_initialized && hasLegacyFiles" class="cleanup-section">
        <h4>🧹 Cleanup</h4>
        <p>Old JSON files detected. Clean them up after confirming everything works.</p>
        <button 
          @click="cleanupLegacyFiles" 
          :disabled="cleaning"
          class="btn-warning"
        >
          {{ cleaning ? 'Cleaning...' : '🗑️ Remove Legacy Files' }}
        </button>
      </div>

      <div v-if="error" class="error-message">
        <h4>❌ Error</h4>
        <p>{{ error }}</p>
        <button @click="clearError" class="btn-secondary">Clear</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface DatabaseInfo {
  database_exists: boolean
  is_initialized: boolean
  chat_sessions_count: number
  conversation_sessions_count: number
  database_size_bytes: number
  database_size_mb: number
}

// State
const loading = ref(true)
const initializing = ref(false)
const cleaning = ref(false)
const dbInfo = ref<DatabaseInfo | null>(null)
const error = ref<string | null>(null)
const hasLegacyFiles = ref(false)

// Methods
async function refreshInfo() {
  try {
    loading.value = true
    error.value = null
    
    dbInfo.value = await invoke<DatabaseInfo>('get_database_info')
    
    // Check for legacy files (simplified check)
    // In a real implementation, you might want to check the app data directory
    hasLegacyFiles.value = false // Set to true if you detect JSON files
    
  } catch (err) {
    error.value = `Failed to get database info: ${err}`
    console.error('Database info error:', err)
  } finally {
    loading.value = false
  }
}

async function initializeDatabase() {
  try {
    initializing.value = true
    error.value = null
    
    const result = await invoke<string>('initialize_database')
    console.log('Database initialized:', result)
    
    // Refresh info after initialization
    await refreshInfo()
    
  } catch (err) {
    error.value = `Failed to initialize database: ${err}`
    console.error('Database initialization error:', err)
  } finally {
    initializing.value = false
  }
}

async function cleanupLegacyFiles() {
  const confirmed = confirm(
    'Are you sure you want to delete legacy JSON files?\n' +
    'Make sure your SQLite database is working properly first!'
  )
  
  if (!confirmed) return
  
  try {
    cleaning.value = true
    error.value = null
    
    const removedFiles = await invoke<string[]>('cleanup_legacy_files', { confirm: true })
    
    if (removedFiles.length > 0) {
      alert(`Cleanup completed!\nRemoved ${removedFiles.length} files: ${removedFiles.join(', ')}`)
    } else {
      alert('No legacy files found to remove.')
    }
    
    hasLegacyFiles.value = false
    
  } catch (err) {
    error.value = `Failed to cleanup legacy files: ${err}`
    console.error('Cleanup error:', err)
  } finally {
    cleaning.value = false
  }
}

function clearError() {
  error.value = null
}

// Initialize
onMounted(() => {
  refreshInfo()
})
</script>

<style scoped>
.database-status {
  max-width: 600px;
  margin: 1rem auto;
}

.status-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 1.5rem;
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.status-header h3 {
  margin: 0;
  color: var(--text-primary);
}

.btn-refresh {
  background: none;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  padding: 0.5rem;
  cursor: pointer;
  font-size: 1rem;
}

.btn-refresh:hover:not(:disabled) {
  background: var(--bg-hover);
}

.btn-refresh:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 2rem;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--border-color);
  border-top: 2px solid var(--accent-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
  margin: 1rem 0;
}

.info-item {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 1rem;
  text-align: center;
}

.info-label {
  font-size: 0.9rem;
  color: var(--text-secondary);
  margin-bottom: 0.5rem;
}

.info-value {
  font-size: 1.2rem;
  font-weight: 600;
  color: var(--text-primary);
}

.info-value.status-good {
  color: var(--accent-primary);
}

.info-value.status-warning {
  color: #ff6b35;
}

.actions {
  margin: 1.5rem 0;
  text-align: center;
}

.cleanup-section {
  margin-top: 2rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-color);
}

.cleanup-section h4 {
  margin: 0 0 0.5rem 0;
  color: var(--text-primary);
}

.cleanup-section p {
  margin: 0 0 1rem 0;
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.btn-primary, .btn-secondary, .btn-warning {
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 6px;
  font-size: 0.9rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.3s;
  margin: 0 0.5rem;
}

.btn-primary {
  background: var(--accent-primary);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: var(--accent-primary-hover);
  transform: translateY(-1px);
}

.btn-secondary {
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover:not(:disabled) {
  background: var(--bg-hover);
}

.btn-warning {
  background: #ff6b35;
  color: white;
}

.btn-warning:hover:not(:disabled) {
  background: #e55a2b;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.error-message {
  background: #fee;
  border: 1px solid #fcc;
  border-radius: 6px;
  padding: 1rem;
  margin: 1rem 0;
}

.error-message h4 {
  margin: 0 0 0.5rem 0;
  color: #d33;
}

.error-message p {
  margin: 0 0 1rem 0;
  color: #a00;
  font-size: 0.9rem;
}
</style>