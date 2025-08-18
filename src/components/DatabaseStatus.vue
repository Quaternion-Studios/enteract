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

      <div class="advanced-section">
        <div class="action-buttons">
          <button @click="checkHealth" :disabled="loadingHealth" class="btn-secondary">
            {{ loadingHealth ? '⏳' : '🔍' }} {{ showHealth ? 'Refresh' : 'Check' }} Health
          </button>
          <button @click="refreshLogs" :disabled="loadingLogs" class="btn-secondary">
            {{ loadingLogs ? '⏳' : '📋' }} {{ showLogs ? 'Refresh' : 'View' }} Logs
          </button>
        </div>

        <div v-if="showHealth && health" class="health-section">
          <h4>🔍 Database Health</h4>
          <div class="health-indicator" :class="{ 'healthy': health.is_healthy, 'unhealthy': !health.is_healthy }">
            <span class="status-icon">{{ health.is_healthy ? '✅' : '❌' }}</span>
            <span class="status-text">{{ health.is_healthy ? 'Healthy' : 'Issues Detected' }}</span>
            <span class="check-time">({{ health.check_duration_ms }}ms)</span>
          </div>

          <div class="health-details">
            <div class="detail-grid">
              <div class="detail-item">
                <span class="label">Connection:</span>
                <span :class="health.can_connect ? 'success' : 'error'">
                  {{ health.can_connect ? 'OK' : 'Failed' }}
                </span>
              </div>
              <div class="detail-item">
                <span class="label">Read/Write:</span>
                <span :class="health.can_read && health.can_write ? 'success' : 'error'">
                  {{ health.can_read && health.can_write ? 'OK' : 'Failed' }}
                </span>
              </div>
              <div class="detail-item">
                <span class="label">Tables:</span>
                <span :class="health.tables_exist ? 'success' : 'error'">
                  {{ health.tables_exist ? 'OK' : 'Missing' }}
                </span>
              </div>
              <div class="detail-item">
                <span class="label">Indexes:</span>
                <span :class="health.indexes_exist ? 'success' : 'warning'">
                  {{ health.indexes_exist ? 'OK' : 'Missing' }}
                </span>
              </div>
              <div class="detail-item">
                <span class="label">WAL Mode:</span>
                <span :class="health.wal_mode ? 'success' : 'warning'">
                  {{ health.wal_mode ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
              <div class="detail-item">
                <span class="label">Foreign Keys:</span>
                <span :class="health.foreign_keys_enabled ? 'success' : 'warning'">
                  {{ health.foreign_keys_enabled ? 'Enabled' : 'Disabled' }}
                </span>
              </div>
            </div>
          </div>

          <div v-if="health.errors.length > 0" class="issues-section errors">
            <h5>❌ Errors:</h5>
            <ul>
              <li v-for="error in health.errors" :key="error">{{ error }}</li>
            </ul>
          </div>

          <div v-if="health.warnings.length > 0" class="issues-section warnings">
            <h5>⚠️ Warnings:</h5>
            <ul>
              <li v-for="warning in health.warnings" :key="warning">{{ warning }}</li>
            </ul>
          </div>
        </div>

        <div v-if="showLogs" class="logs-section">
          <div class="logs-header">
            <h4>📋 Database Logs</h4>
            <div class="log-controls">
              <button @click="refreshLogs" :disabled="loadingLogs" class="btn-sm">
                {{ loadingLogs ? '⏳' : '🔄' }}
              </button>
              <button @click="clearLogs" :disabled="loadingLogs" class="btn-sm">
                🗑️
              </button>
            </div>
          </div>
          
          <div v-if="logs.length > 0" class="logs-list">
            <div v-for="log in logs.slice(-20)" :key="log.timestamp" 
                 class="log-entry" 
                 :class="logLevelClass(log.level)">
              <div class="log-time">{{ formatTime(log.timestamp) }}</div>
              <div class="log-operation">{{ log.operation }}</div>
              <div class="log-message">{{ log.message }}</div>
              <div v-if="log.duration_ms" class="log-duration">{{ log.duration_ms }}ms</div>
            </div>
          </div>
          <div v-else class="no-logs">No logs available</div>
        </div>
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

interface DatabaseHealth {
  is_healthy: boolean
  can_connect: boolean
  can_read: boolean
  can_write: boolean
  foreign_keys_enabled: boolean
  wal_mode: boolean
  tables_exist: boolean
  indexes_exist: boolean
  path_accessible: boolean
  directory_writable: boolean
  last_check: number
  check_duration_ms: number
  errors: string[]
  warnings: string[]
}

interface LogEntry {
  level: string
  timestamp: number
  operation: string
  message: string
  details?: any
  duration_ms?: number
  session_id?: string
}

// State
const loading = ref(true)
const initializing = ref(false)
const cleaning = ref(false)
const loadingHealth = ref(false)
const loadingLogs = ref(false)
const dbInfo = ref<DatabaseInfo | null>(null)
const health = ref<DatabaseHealth | null>(null)
const logs = ref<LogEntry[]>([])
const error = ref<string | null>(null)
const hasLegacyFiles = ref(false)
const showHealth = ref(false)
const showLogs = ref(false)

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

async function checkHealth() {
  try {
    loadingHealth.value = true
    error.value = null
    
    health.value = await invoke<DatabaseHealth>('check_database_health')
    showHealth.value = true
    
  } catch (err) {
    error.value = `Failed to check database health: ${err}`
    console.error('Database health check error:', err)
  } finally {
    loadingHealth.value = false
  }
}

async function refreshLogs() {
  try {
    loadingLogs.value = true
    error.value = null
    
    logs.value = await invoke<LogEntry[]>('get_database_logs', { lastN: 50 })
    showLogs.value = true
    
  } catch (err) {
    error.value = `Failed to get database logs: ${err}`
    console.error('Database logs error:', err)
  } finally {
    loadingLogs.value = false
  }
}

async function clearLogs() {
  try {
    loadingLogs.value = true
    error.value = null
    
    await invoke('clear_database_logs')
    logs.value = []
    
  } catch (err) {
    error.value = `Failed to clear database logs: ${err}`
    console.error('Clear logs error:', err)
  } finally {
    loadingLogs.value = false
  }
}

function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString()
}

function logLevelClass(level: string): string {
  switch (level.toLowerCase()) {
    case 'error':
    case 'critical':
      return 'log-error'
    case 'warn':
      return 'log-warn'
    case 'info':
      return 'log-info'
    case 'debug':
    case 'trace':
      return 'log-debug'
    default:
      return ''
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

.advanced-section {
  margin-top: 2rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-color);
}

.action-buttons {
  display: flex;
  gap: 1rem;
  margin-bottom: 1.5rem;
  justify-content: center;
}

.health-section, .logs-section {
  margin-top: 1.5rem;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 1rem;
}

.health-indicator {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 1rem;
  padding: 0.75rem;
  border-radius: 6px;
}

.health-indicator.healthy {
  background: rgba(0, 255, 0, 0.1);
  border: 1px solid rgba(0, 255, 0, 0.3);
}

.health-indicator.unhealthy {
  background: rgba(255, 0, 0, 0.1);
  border: 1px solid rgba(255, 0, 0, 0.3);
}

.status-icon {
  font-size: 1.2rem;
}

.status-text {
  font-weight: 600;
  color: var(--text-primary);
}

.check-time {
  color: var(--text-secondary);
  font-size: 0.9rem;
  margin-left: auto;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.detail-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 0.9rem;
}

.detail-item .label {
  color: var(--text-secondary);
}

.success {
  color: #4ade80;
  font-weight: 500;
}

.warning {
  color: #fbbf24;
  font-weight: 500;
}

.error {
  color: #f87171;
  font-weight: 500;
}

.issues-section {
  margin-top: 1rem;
  padding: 0.75rem;
  border-radius: 6px;
}

.issues-section.errors {
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.3);
}

.issues-section.warnings {
  background: rgba(251, 191, 36, 0.1);
  border: 1px solid rgba(251, 191, 36, 0.3);
}

.issues-section h5 {
  margin: 0 0 0.5rem 0;
  font-size: 0.9rem;
}

.issues-section ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.issues-section li {
  padding: 0.25rem 0;
  font-size: 0.85rem;
  color: var(--text-primary);
}

.logs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.logs-header h4 {
  margin: 0;
  color: var(--text-primary);
}

.log-controls {
  display: flex;
  gap: 0.5rem;
}

.btn-sm {
  padding: 0.4rem 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  cursor: pointer;
  font-size: 0.8rem;
  transition: background 0.2s;
}

.btn-sm:hover:not(:disabled) {
  background: var(--bg-hover);
}

.btn-sm:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.logs-list {
  max-height: 250px;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  background: var(--bg-secondary);
}

.log-entry {
  display: grid;
  grid-template-columns: auto 120px 1fr auto;
  gap: 0.75rem;
  padding: 0.5rem;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.8rem;
  align-items: center;
}

.log-entry:last-child {
  border-bottom: none;
}

.log-error {
  background: rgba(248, 113, 113, 0.05);
  border-left: 3px solid #f87171;
}

.log-warn {
  background: rgba(251, 191, 36, 0.05);
  border-left: 3px solid #fbbf24;
}

.log-info {
  background: rgba(59, 130, 246, 0.05);
  border-left: 3px solid #3b82f6;
}

.log-debug {
  background: rgba(156, 163, 175, 0.05);
  border-left: 3px solid #9ca3af;
}

.log-time {
  color: var(--text-secondary);
  font-family: monospace;
}

.log-operation {
  color: #60a5fa;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.log-message {
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
}

.log-duration {
  color: #10b981;
  text-align: right;
  font-family: monospace;
  white-space: nowrap;
}

.no-logs {
  text-align: center;
  padding: 2rem;
  color: var(--text-secondary);
  font-style: italic;
}
</style>