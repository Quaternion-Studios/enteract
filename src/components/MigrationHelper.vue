<template>
  <div class="migration-helper">
    <!-- Migration Status Card -->
    <div class="migration-card">
      <div class="migration-header">
        <h2>🚀 Database Migration</h2>
        <p class="subtitle">Upgrade to SQLite for better performance</p>
      </div>

      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>Checking migration status...</p>
      </div>

      <div v-else-if="migrationStatus" class="migration-content">
        <!-- Status Overview -->
        <div class="status-overview">
          <div class="status-item" :class="{ active: migrationStatus.database_exists }">
            <div class="status-icon">{{ migrationStatus.database_exists ? '✅' : '⏳' }}</div>
            <div class="status-text">SQLite Database</div>
          </div>
          <div class="status-item" :class="{ active: migrationStatus.has_json_data }">
            <div class="status-icon">{{ migrationStatus.has_json_data ? '📁' : '❌' }}</div>
            <div class="status-text">JSON Data Found</div>
          </div>
          <div class="status-item" :class="{ active: migrationStatus.migration_completed }">
            <div class="status-icon">{{ migrationStatus.migration_completed ? '✅' : '⏳' }}</div>
            <div class="status-text">Migration Complete</div>
          </div>
        </div>

        <!-- Migration Actions -->
        <div v-if="migrationStatus.needs_migration" class="migration-actions">
          <div class="migration-info">
            <h3>🔄 Migration Required</h3>
            <p>Your data is currently stored in JSON files. Migrate to SQLite for:</p>
            <ul>
              <li>⚡ Faster data access and queries</li>
              <li>💾 Reduced memory usage</li>
              <li>🔍 Better search capabilities</li>
              <li>📈 Improved scalability</li>
            </ul>
          </div>

          <div class="action-buttons">
            <button 
              @click="performMigration" 
              :disabled="migrating" 
              class="btn-primary"
            >
              {{ migrating ? 'Migrating...' : '🚀 Start Migration' }}
            </button>
            
            <button 
              @click="createBackup" 
              :disabled="creatingBackup"
              class="btn-secondary"
            >
              {{ creatingBackup ? 'Backing up...' : '💾 Backup Data First' }}
            </button>
          </div>

          <!-- Migration Progress -->
          <div v-if="migrating" class="migration-progress">
            <div class="progress-bar">
              <div class="progress-fill" :style="{ width: migrationProgress + '%' }"></div>
            </div>
            <p class="progress-text">{{ migrationMessage }}</p>
          </div>
        </div>

        <!-- Post-Migration Actions -->
        <div v-else-if="migrationStatus.migration_completed" class="post-migration">
          <div class="success-message">
            <h3>✅ Migration Completed!</h3>
            <p>Your data has been successfully migrated to SQLite.</p>
          </div>

          <!-- SQLite Stats -->
          <div v-if="sqliteStats" class="stats-grid">
            <div class="stat-card">
              <div class="stat-number">{{ sqliteStats.chat_sessions }}</div>
              <div class="stat-label">Chat Sessions</div>
            </div>
            <div class="stat-card">
              <div class="stat-number">{{ sqliteStats.chat_messages }}</div>
              <div class="stat-label">Chat Messages</div>
            </div>
            <div class="stat-card">
              <div class="stat-number">{{ sqliteStats.conversation_sessions }}</div>
              <div class="stat-label">Conversations</div>
            </div>
            <div class="stat-card">
              <div class="stat-number">{{ sqliteStats.database_size_mb.toFixed(2) }}MB</div>
              <div class="stat-label">Database Size</div>
            </div>
          </div>

          <!-- Cleanup Actions -->
          <div class="cleanup-actions">
            <h4>🧹 Cleanup Options</h4>
            <div class="action-buttons">
              <button 
                @click="cleanupJsonFiles" 
                :disabled="cleaningUp"
                class="btn-warning"
              >
                {{ cleaningUp ? 'Cleaning up...' : '🗑️ Remove Old JSON Files' }}
              </button>
              <button @click="refreshStats" class="btn-secondary">
                🔄 Refresh Stats
              </button>
            </div>
          </div>
        </div>

        <!-- No Migration Needed -->
        <div v-else class="no-migration">
          <h3>✅ All Set!</h3>
          <p>{{ migrationStatus.database_exists ? 'SQLite database is ready.' : 'No data to migrate.' }}</p>
        </div>
      </div>

      <!-- Migration Results -->
      <div v-if="migrationResult && migrationResult.success" class="migration-results">
        <h3>📊 Migration Results</h3>
        <div class="results-grid">
          <div class="result-item">
            <span class="result-label">Chat Sessions:</span>
            <span class="result-value">{{ migrationResult.result?.chat_sessions_migrated || 0 }}</span>
          </div>
          <div class="result-item">
            <span class="result-label">Chat Messages:</span>
            <span class="result-value">{{ migrationResult.result?.chat_messages_migrated || 0 }}</span>
          </div>
          <div class="result-item">
            <span class="result-label">Conversations:</span>
            <span class="result-value">{{ migrationResult.result?.conversation_sessions_migrated || 0 }}</span>
          </div>
          <div class="result-item">
            <span class="result-label">Total Records:</span>
            <span class="result-value total">{{ migrationResult.result ? 
              (migrationResult.result.chat_sessions_migrated + 
               migrationResult.result.chat_messages_migrated + 
               migrationResult.result.conversation_sessions_migrated) : 0 }}</span>
          </div>
        </div>
      </div>

      <!-- Error Display -->
      <div v-if="error" class="error-message">
        <h3>❌ Error</h3>
        <p>{{ error }}</p>
        <button @click="checkStatus" class="btn-secondary">🔄 Retry</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Types
interface MigrationStatus {
  is_sqlite_enabled: boolean
  has_json_data: boolean
  needs_migration: boolean
  migration_completed: boolean
  database_exists: boolean
}

interface SqliteStats {
  chat_sessions: number
  chat_messages: number
  conversation_sessions: number
  conversation_messages: number
  conversation_insights: number
  database_size_bytes: number
  database_size_mb: number
}

interface MigrationResult {
  success: boolean
  message: string
  result?: {
    chat_sessions_migrated: number
    chat_messages_migrated: number
    conversation_sessions_migrated: number
    conversation_messages_migrated: number
    conversation_insights_migrated: number
  }
  error?: string
}

// State
const loading = ref(true)
const migrationStatus = ref<MigrationStatus | null>(null)
const sqliteStats = ref<SqliteStats | null>(null)
const migrationResult = ref<MigrationResult | null>(null)
const error = ref<string | null>(null)

// Loading states
const migrating = ref(false)
const creatingBackup = ref(false)
const cleaningUp = ref(false)
const migrationProgress = ref(0)
const migrationMessage = ref('')

// Methods
async function checkStatus() {
  try {
    loading.value = true
    error.value = null
    
    migrationStatus.value = await invoke<MigrationStatus>('check_migration_status')
    
    if (migrationStatus.value.migration_completed) {
      await loadSqliteStats()
    }
  } catch (err) {
    error.value = `Failed to check migration status: ${err}`
    console.error('Migration status check failed:', err)
  } finally {
    loading.value = false
  }
}

async function loadSqliteStats() {
  try {
    sqliteStats.value = await invoke<SqliteStats>('get_sqlite_stats')
  } catch (err) {
    console.error('Failed to load SQLite stats:', err)
  }
}

async function createBackup() {
  try {
    creatingBackup.value = true
    const backupFiles = await invoke<string[]>('backup_json_files')
    
    console.log('Backup created:', backupFiles)
    alert(`Backup created successfully!\nFiles saved: ${backupFiles.length}`)
  } catch (err) {
    error.value = `Failed to create backup: ${err}`
    console.error('Backup failed:', err)
  } finally {
    creatingBackup.value = false
  }
}

async function performMigration() {
  try {
    migrating.value = true
    migrationProgress.value = 0
    migrationMessage.value = 'Starting migration...'
    
    // Simulate progress updates
    const progressInterval = setInterval(() => {
      if (migrationProgress.value < 90) {
        migrationProgress.value += 10
        migrationMessage.value = `Migrating data... ${migrationProgress.value}%`
      }
    }, 500)
    
    migrationResult.value = await invoke<MigrationResult>('migrate_to_sqlite')
    
    clearInterval(progressInterval)
    migrationProgress.value = 100
    migrationMessage.value = 'Migration completed!'
    
    // Refresh status
    await checkStatus()
    
    if (migrationResult.value.success) {
      alert('Migration completed successfully! 🎉')
    }
  } catch (err) {
    error.value = `Migration failed: ${err}`
    console.error('Migration failed:', err)
  } finally {
    migrating.value = false
  }
}

async function cleanupJsonFiles() {
  const confirmed = confirm(
    'Are you sure you want to delete the old JSON files?\n' +
    'This action cannot be undone. Make sure you have a backup!'
  )
  
  if (!confirmed) return
  
  try {
    cleaningUp.value = true
    const removedFiles = await invoke<string[]>('cleanup_json_files', { confirm: true })
    
    console.log('Removed files:', removedFiles)
    alert(`Cleanup completed!\nRemoved ${removedFiles.length} files`)
    
    // Refresh status
    await checkStatus()
  } catch (err) {
    error.value = `Cleanup failed: ${err}`
    console.error('Cleanup failed:', err)
  } finally {
    cleaningUp.value = false
  }
}

async function refreshStats() {
  await loadSqliteStats()
}

// Initialize
onMounted(() => {
  checkStatus()
})
</script>

<style scoped>
.migration-helper {
  max-width: 800px;
  margin: 2rem auto;
  padding: 0 1rem;
}

.migration-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 2rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.migration-header {
  text-align: center;
  margin-bottom: 2rem;
}

.migration-header h2 {
  color: var(--text-primary);
  margin: 0 0 0.5rem 0;
  font-size: 1.8rem;
}

.subtitle {
  color: var(--text-secondary);
  margin: 0;
  font-size: 1rem;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 2rem;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border-color);
  border-top: 3px solid var(--accent-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.status-overview {
  display: flex;
  justify-content: space-around;
  margin-bottom: 2rem;
  padding: 1rem;
  background: var(--bg-primary);
  border-radius: 8px;
}

.status-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  opacity: 0.5;
  transition: opacity 0.3s;
}

.status-item.active {
  opacity: 1;
}

.status-icon {
  font-size: 2rem;
  margin-bottom: 0.5rem;
}

.status-text {
  font-size: 0.9rem;
  color: var(--text-secondary);
}

.migration-info {
  margin-bottom: 1.5rem;
}

.migration-info h3 {
  color: var(--text-primary);
  margin: 0 0 1rem 0;
}

.migration-info ul {
  margin: 1rem 0;
  padding-left: 1.5rem;
}

.migration-info li {
  margin: 0.5rem 0;
  color: var(--text-secondary);
}

.action-buttons {
  display: flex;
  gap: 1rem;
  margin: 1.5rem 0;
  flex-wrap: wrap;
}

.btn-primary, .btn-secondary, .btn-warning {
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 6px;
  font-size: 0.9rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.3s;
  flex: 1;
  min-width: 150px;
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
}

.migration-progress {
  margin: 1rem 0;
}

.progress-bar {
  width: 100%;
  height: 8px;
  background: var(--bg-primary);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 0.5rem;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary));
  transition: width 0.3s;
  border-radius: 4px;
}

.progress-text {
  text-align: center;
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 1rem;
  margin: 1.5rem 0;
}

.stat-card {
  background: var(--bg-primary);
  padding: 1rem;
  border-radius: 8px;
  text-align: center;
  border: 1px solid var(--border-color);
}

.stat-number {
  font-size: 1.5rem;
  font-weight: bold;
  color: var(--accent-primary);
  margin-bottom: 0.25rem;
}

.stat-label {
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.results-grid {
  display: grid;
  gap: 0.5rem;
  margin: 1rem 0;
}

.result-item {
  display: flex;
  justify-content: space-between;
  padding: 0.5rem;
  background: var(--bg-primary);
  border-radius: 4px;
}

.result-label {
  color: var(--text-secondary);
}

.result-value {
  color: var(--text-primary);
  font-weight: 500;
}

.result-value.total {
  color: var(--accent-primary);
  font-weight: bold;
}

.success-message {
  text-align: center;
  margin-bottom: 2rem;
}

.success-message h3 {
  color: var(--accent-primary);
  margin: 0 0 0.5rem 0;
}

.cleanup-actions {
  margin-top: 2rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--border-color);
}

.cleanup-actions h4 {
  color: var(--text-primary);
  margin: 0 0 1rem 0;
}

.no-migration {
  text-align: center;
  padding: 2rem;
}

.no-migration h3 {
  color: var(--accent-primary);
  margin: 0 0 1rem 0;
}

.error-message {
  background: #fee;
  border: 1px solid #fcc;
  border-radius: 8px;
  padding: 1.5rem;
  margin: 1rem 0;
}

.error-message h3 {
  color: #d33;
  margin: 0 0 1rem 0;
}

.error-message p {
  color: #a00;
  margin: 0 0 1rem 0;
}
</style>