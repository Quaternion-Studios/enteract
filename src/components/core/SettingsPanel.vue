<script setup lang="ts">
import { ref, watch, onMounted, nextTick } from 'vue'
import {
  Cog6ToothIcon,
  XMarkIcon,
  SpeakerWaveIcon,
  MicrophoneIcon,
  ComputerDesktopIcon,
  CpuChipIcon,
  DocumentTextIcon
} from '@heroicons/vue/24/outline'
import { useAIModels } from '../../composables/useAIModels'
import { useTransparency } from '../../composables/useTransparency'
import { useWindowRegistration } from '../../composables/useWindowRegistry'
import { useRagDocuments } from '../../composables/rag'
import { invoke } from '@tauri-apps/api/core'
import ModelsTab from './settings/ModelsTab.vue'
import AudioTab from './settings/AudioTab.vue'
import DocumentsTab from './settings/DocumentsTab.vue'
import GeneralTab from './settings/GeneralTab.vue'

interface Props {
  showSettingsPanel: boolean
}

interface Emits {
  (e: 'close'): void
  (e: 'update:showSettingsPanel', value: boolean): void
}

// Audio Device Types from Rust implementation
interface AudioLoopbackDevice {
  id: string
  name: string
  is_default: boolean
  sample_rate: number
  channels: number
  format: string
  device_type: 'Render' | 'Capture'
  loopback_method: 'RenderLoopback' | 'CaptureDevice' | 'StereoMix'
}

interface AudioDeviceSettings {
  selectedLoopbackDevice: string | null
  loopbackEnabled: boolean
  bufferSize: number
  sampleRate: number
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Window registry for centralized window management
const windowRegistry = useWindowRegistration('settings-panel', {
  closeOnClickOutside: false, // Temporarily disabled for testing
  isModal: true, // Settings panel should be modal
  priority: 100, // Lower priority than other windows
  closeHandler: () => closePanel()
})

// Components refs
const settingsPanelRef = ref<HTMLElement>()

// Settings tabs
const activeTab = ref<'models' | 'audio' | 'documents' | 'general'>('models')

// RAG Documents system
const ragDocuments = useRagDocuments()
const isDragOver = ref(false)
const isUploading = ref(false)

// AI Models (existing functionality)
const {
  ollamaModels,
  ollamaStatus,
  isLoadingModels,
  modelsError,
  selectedModel,
  pullingModel,
  deletingModel,
  fetchOllamaStatus,
  fetchOllamaModels,
  pullModel,
  deleteModel,
  formatModelSize,
  getModelDisplayName
} = useAIModels()

// Audio Loopback Settings
const audioDevices = ref<AudioLoopbackDevice[]>([])
const isLoadingAudioDevices = ref(false)
const audioDevicesError = ref<string | null>(null)
const testingDeviceId = ref<string | null>(null)
const audioSettings = ref<AudioDeviceSettings>({
  selectedLoopbackDevice: null,
  loopbackEnabled: false,
  bufferSize: 4096,
  sampleRate: 16000
})

// General Settings
const generalSettings = ref({
  theme: 'dark',
  autoStartOllama: false,
  enableNotifications: true,
  logLevel: 'info',
  startMinimized: false,
  startWithSystem: false,
  saveWindowPosition: true,
  enableKeyboardShortcuts: true,
  transcriptionLanguage: 'en',
  enableAutoSave: true,
  autoSaveInterval: 5,
  // Whisper model settings
  microphoneWhisperModel: 'tiny',
  loopbackWhisperModel: 'small',
  // Transparency settings
  enableTransparency: false,
  defaultTransparencyLevel: 1.0,
  autoRestoreOnError: true
})

// System Information
interface GpuInfo {
  name: string
  vendor: string
  driver_version?: string
  memory_mb?: number
  temperature_celsius?: number
  utilization_percent?: number
}

interface SystemInfo {
  gpus: GpuInfo[]
  cpu_name: string
  memory_gb: number
  os: string
}

const systemInfo = ref<SystemInfo | null>(null)
const isLoadingSystemInfo = ref(false)
const systemInfoError = ref<string | null>(null)

// Transparency composable
const transparency = useTransparency()

// Audio device enumeration functions
const enumerateAudioDevices = async () => {
  isLoadingAudioDevices.value = true
  audioDevicesError.value = null
  
  try {
    // Call Rust function to enumerate loopback devices
    const devices = await invoke<AudioLoopbackDevice[]>('enumerate_loopback_devices')
    audioDevices.value = devices
    console.log('🔊 Found audio loopback devices:', devices)
    
    // Auto-select the best device if none selected
    if (!audioSettings.value.selectedLoopbackDevice && devices.length > 0) {
      const bestDevice = await invoke<AudioLoopbackDevice | null>('auto_select_best_device')
      if (bestDevice) {
        audioSettings.value.selectedLoopbackDevice = bestDevice.id
        console.log('🎯 Auto-selected audio device:', bestDevice.name)
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    audioDevicesError.value = message
    console.error('Failed to enumerate audio devices:', error)
  } finally {
    isLoadingAudioDevices.value = false
  }
}

const testAudioDevice = async (deviceId: string) => {
  try {
    const result = await invoke<boolean>('test_audio_device', { deviceId })
    if (result) {
      console.log('✅ Audio device test successful')
    } else {
      console.log('❌ Audio device test failed')
    }
    return result
  } catch (error) {
    console.error('Audio device test error:', error)
    return false
  }
}

const getDeviceIcon = (device: AudioLoopbackDevice) => {
  if (device.device_type === 'Render') {
    return SpeakerWaveIcon
  } else if (device.loopback_method === 'StereoMix') {
    return ComputerDesktopIcon
  } else {
    return MicrophoneIcon
  }
}

const getDeviceMethodBadge = (method: string) => {
  switch (method) {
    case 'RenderLoopback':
      return { text: 'Render Loopback', class: 'bg-green-500/20 text-green-300 border-green-400/30' }
    case 'StereoMix':
      return { text: 'Stereo Mix', class: 'bg-blue-500/20 text-blue-300 border-blue-400/30' }
    case 'CaptureDevice':
      return { text: 'Capture Device', class: 'bg-purple-500/20 text-purple-300 border-purple-400/30' }
    default:
      return { text: method, class: 'bg-gray-500/20 text-gray-300 border-gray-400/30' }
  }
}

// Load settings from storage
const loadSettings = async () => {
  try {
    const storedAudioSettings = await invoke<AudioDeviceSettings | null>('load_audio_settings')
    if (storedAudioSettings) {
      // Make sure to preserve the loaded selected device
      audioSettings.value = {
        ...audioSettings.value,
        ...storedAudioSettings
      }
      console.log('📂 Loaded audio settings:', storedAudioSettings)
    }
    
    const storedGeneralSettings = await invoke<any>('load_general_settings')
    if (storedGeneralSettings) {
      generalSettings.value = { ...generalSettings.value, ...storedGeneralSettings }
      
      // Apply transparency settings from loaded settings
      applyTransparencyFromSettings()
    }
  } catch (error) {
    console.error('Failed to load settings:', error)
  }
}

// Save settings to storage
const saveAudioSettings = async () => {
  try {
    await invoke('save_audio_settings', { settings: audioSettings.value })
    console.log('💾 Audio settings saved')
  } catch (error) {
    console.error('Failed to save audio settings:', error)
  }
}

const saveGeneralSettings = async () => {
  try {
    await invoke('save_general_settings', { settings: generalSettings.value })
    console.log('💾 General settings saved')
  } catch (error) {
    console.error('Failed to save general settings:', error)
  }
}

// Watch for settings panel state changes
watch(() => props.showSettingsPanel, async (newValue) => {
  if (newValue) {
    await nextTick()
    
    // Register the settings panel element when it opens
    if (settingsPanelRef.value) {
      windowRegistry.registerSelf(settingsPanelRef.value)
    }
    
    await loadSettings()
    
    // Load AI models if on models tab
    if (activeTab.value === 'models') {
      await fetchOllamaStatus()
      if (ollamaStatus.value.status === 'running') {
        await fetchOllamaModels()
      }
    }
    
    // Load audio devices if on audio tab
    if (activeTab.value === 'audio') {
      await enumerateAudioDevices()
    }
    
    // Initialize RAG system if on documents tab
    if (activeTab.value === 'documents') {
      await ragDocuments.initialize()
    }
  } else {
    // Unregister when settings panel closes
    windowRegistry.unregisterSelf()
  }
})

// Watch for tab changes
watch(activeTab, async (newTab) => {
  if (!props.showSettingsPanel) return
  
  if (newTab === 'models') {
    await fetchOllamaStatus()
    if (ollamaStatus.value.status === 'running') {
      await fetchOllamaModels()
    }
  } else if (newTab === 'audio') {
    await enumerateAudioDevices()
  } else if (newTab === 'documents') {
    await ragDocuments.initialize()
  }
})

// Watch audio settings changes and auto-save
watch(audioSettings, () => {
  saveAudioSettings()
}, { deep: true })

// Watch general settings changes and auto-save
watch(generalSettings, (newSettings, oldSettings) => {
  console.log('🔧 General settings watcher triggered:', {
    newLevel: newSettings.defaultTransparencyLevel,
    oldLevel: oldSettings?.defaultTransparencyLevel,
    newEnabled: newSettings.enableTransparency,
    oldEnabled: oldSettings?.enableTransparency
  })
  
  // Check if whisper model settings changed and emit event for reinitialization
  if (oldSettings && (
    newSettings.microphoneWhisperModel !== oldSettings.microphoneWhisperModel ||
    newSettings.loopbackWhisperModel !== oldSettings.loopbackWhisperModel
  )) {
    console.log('🔄 Whisper model settings changed, emitting reinitialize event')
    
    // Emit custom event that speech transcription can listen to
    const reinitializeEvent = new CustomEvent('whisper-models-changed', {
      detail: {
        microphoneModel: newSettings.microphoneWhisperModel,
        loopbackModel: newSettings.loopbackWhisperModel
      }
    })
    window.dispatchEvent(reinitializeEvent)
  }
  
  // Handle transparency enable/disable changes
  if (oldSettings && newSettings.enableTransparency !== oldSettings.enableTransparency) {
    console.log('🔧 Transparency enable/disable changed:', {
      oldEnabled: oldSettings.enableTransparency,
      newEnabled: newSettings.enableTransparency
    })
    applyTransparencyFromSettings()
  }
  
  // Auto-save all general settings changes
  saveGeneralSettings()
}, { deep: true })



// Function to apply transparency settings from general settings
const applyTransparencyFromSettings = () => {
  console.log('🔧 Applying transparency from settings:', {
    enabled: generalSettings.value.enableTransparency,
    level: generalSettings.value.defaultTransparencyLevel
  })
  
  if (generalSettings.value.enableTransparency) {
    transparency.setLevel(generalSettings.value.defaultTransparencyLevel)
  } else {
    transparency.presets.solid()
  }
}


// Handler for transparency level slider changes
const handleTransparencyLevelChange = () => {
  console.log('🔧 Transparency level changed via slider:', generalSettings.value.defaultTransparencyLevel)
  if (generalSettings.value.enableTransparency) {
    transparency.setLevel(generalSettings.value.defaultTransparencyLevel)
  }
  saveGeneralSettings()
}

const closePanel = () => {
  emit('close')
  emit('update:showSettingsPanel', false)
}

const clearModelsError = () => {
  modelsError.value = null
}

const clearAudioError = () => {
  audioDevicesError.value = null
}

const selectAudioDevice = async (deviceId: string) => {
  console.log('🔊 selectAudioDevice called with:', deviceId)
  
  // Check if already selected
  if (audioSettings.value.selectedLoopbackDevice === deviceId) {
    console.log('🔊 Device already selected:', deviceId)
    return
  }
  
  // Clear any previous errors
  audioDevicesError.value = null
  
  // Set testing state
  testingDeviceId.value = deviceId
  
  try {
    // For now, let's skip the test and directly select the device
    // This will help us determine if the issue is with the test function
    console.log('🔊 Selecting audio device:', deviceId)
    
    // Update the selected device directly
    audioSettings.value.selectedLoopbackDevice = deviceId
    console.log('🔊 Successfully selected audio device:', deviceId)
    
    // Save settings immediately
    await saveAudioSettings()
    
    // Optional: Test the device after selection (non-blocking)
    testAudioDevice(deviceId).then(testResult => {
      if (!testResult) {
        console.warn('⚠️ Audio device test failed, but device remains selected:', deviceId)
      } else {
        console.log('✅ Audio device test passed:', deviceId)
      }
    }).catch(error => {
      console.error('❌ Audio device test error:', error)
    })
    
  } catch (error) {
    console.error('❌ Error selecting audio device:', error)
    audioDevicesError.value = `Error: ${error instanceof Error ? error.message : 'Failed to select device'}`
  } finally {
    // Clear testing state
    testingDeviceId.value = null
  }
}

// Local handlers for child interactions
const handleSelectModel = (name: string) => {
  selectedModel.value = name
}

const setAudioLoopbackEnabled = (value: boolean) => {
  audioSettings.value.loopbackEnabled = value
}

const setBufferSize = (value: number) => {
  audioSettings.value.bufferSize = value
}

const setSampleRate = (value: number) => {
  audioSettings.value.sampleRate = value
}

onMounted(() => {
  loadSettings()
  fetchSystemInfo()
})

// Fetch system information
const fetchSystemInfo = async () => {
  isLoadingSystemInfo.value = true
  systemInfoError.value = null
  
  try {
    const info = await invoke<SystemInfo>('get_system_info')
    systemInfo.value = info
    console.log('💻 System info loaded:', info)
  } catch (error) {
    console.error('Failed to fetch system info:', error)
    systemInfoError.value = error instanceof Error ? error.message : String(error)
  } finally {
    isLoadingSystemInfo.value = false
  }
}

// Format GPU memory
const formatGpuMemory = (memoryMb?: number): string => {
  if (!memoryMb) return 'Unknown'
  if (memoryMb >= 1024) {
    return `${(memoryMb / 1024).toFixed(1)} GB`
  }
  return `${memoryMb} MB`
}

const handleFileUpload = async (event: Event) => {
  const input = event.target as HTMLInputElement
  const files = input.files
  if (!files) return
  
  isUploading.value = true
  try {
    await ragDocuments.uploadDocuments(files)
  } catch (error) {
    console.error('Error uploading documents:', error)
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
    await ragDocuments.uploadDocuments(files)
  } catch (error) {
    console.error('Error uploading documents:', error)
  } finally {
    isUploading.value = false
  }
}

const deleteDocument = async (documentId: string) => {
  try {
    await ragDocuments.deleteDocument(documentId)
  } catch (error) {
    console.error('Error deleting document:', error)
  }
}

const toggleDocumentSelection = (documentId: string) => {
  ragDocuments.toggleDocumentSelection(documentId)
}

const clearAllSelections = () => {
  ragDocuments.clearSelection()
}

const generateEmbeddings = async (documentId: string) => {
  try {
    await ragDocuments.generateEmbeddings(documentId)
  } catch (error) {
    console.error('Error generating embeddings:', error)
  }
}

const clearCache = async () => {
  try {
    await ragDocuments.clearEmbeddingCache()
  } catch (error) {
    console.error('Error clearing cache:', error)
  }
}

const getDocumentIcon = (fileType: string) => {
  if (fileType.includes('pdf')) return '📄'
  if (fileType.includes('image')) return '🖼️'
  if (fileType.includes('text')) return '📝'
  if (fileType.includes('doc')) return '📃'
  return '📎'
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <Transition name="settings-drawer">
    <div v-if="showSettingsPanel" ref="settingsPanelRef" class="settings-drawer">
        <!-- Drawer Header -->
        <div class="drawer-header">
          <div class="drawer-title">
            <Cog6ToothIcon class="w-5 h-5 text-white/90" />
            <span class="text-lg font-semibold text-white">Settings</span>
          </div>
          <button @click="closePanel" class="drawer-close-btn">
            <XMarkIcon class="w-5 h-5 text-white/70 hover:text-white transition-colors" />
          </button>
        </div>
        
        <!-- Drawer Body -->
        <div class="drawer-body">
          <!-- Settings Navigation -->
          <div class="settings-nav">
            <button
              @click="activeTab = 'models'"
              :class="{ 'nav-active': activeTab === 'models' }"
              class="nav-item"
            >
              <div class="nav-icon">
                <CpuChipIcon class="w-5 h-5" />
              </div>
              <div class="nav-content">
                <div class="nav-title">AI Models</div>
                <div class="nav-subtitle">Manage models</div>
              </div>
              <div class="nav-status" :class="{
                'status-success': ollamaStatus.status === 'running',
                'status-error': ollamaStatus.status === 'not_running',
                'status-warning': ollamaStatus.status === 'checking' || ollamaStatus.status === 'error'
              }">
                <div class="status-dot"></div>
              </div>
            </button>
            
            <button
              @click="activeTab = 'audio'"
              :class="{ 'nav-active': activeTab === 'audio' }"
              class="nav-item"
            >
              <div class="nav-icon">
                <SpeakerWaveIcon class="w-5 h-5" />
              </div>
              <div class="nav-content">
                <div class="nav-title">Audio Loopback</div>
                <div class="nav-subtitle">System audio capture</div>
              </div>
              <div class="nav-status" :class="{
                'status-success': audioSettings.loopbackEnabled && audioSettings.selectedLoopbackDevice,
                'status-error': !audioSettings.loopbackEnabled || !audioSettings.selectedLoopbackDevice,
                'status-warning': isLoadingAudioDevices
              }">
                <div class="status-dot"></div>
              </div>
            </button>
            
            <button
              @click="activeTab = 'documents'"
              :class="{ 'nav-active': activeTab === 'documents' }"
              class="nav-item"
            >
              <div class="nav-icon">
                <DocumentTextIcon class="w-5 h-5" />
              </div>
              <div class="nav-content">
                <div class="nav-title">Documents</div>
                <div class="nav-subtitle">RAG knowledge base</div>
              </div>
              <div class="nav-status" :class="{
                'status-success': ragDocuments.documents.value.length > 0 && ragDocuments.selectedDocumentIds.value.size > 0,
                'status-warning': ragDocuments.documents.value.length > 0 && ragDocuments.selectedDocumentIds.value.size === 0,
                'status-error': ragDocuments.documents.value.length === 0
              }">
                <div class="status-dot"></div>
              </div>
            </button>
            
            <button
              @click="activeTab = 'general'"
              :class="{ 'nav-active': activeTab === 'general' }"
              class="nav-item"
            >
              <div class="nav-icon">
                <Cog6ToothIcon class="w-5 h-5" />
              </div>
              <div class="nav-content">
                <div class="nav-title">General</div>
                <div class="nav-subtitle">App preferences</div>
              </div>
            </button>
          </div>
          
          <!-- Settings Content -->
          <div class="settings-content">
            <!-- AI Models Tab -->
            <ModelsTab
              v-if="activeTab === 'models'"
              :ollama-status="ollamaStatus"
              :is-loading-models="isLoadingModels"
              :models-error="modelsError"
              :ollama-models="ollamaModels"
              :selected-model="selectedModel"
              :pulling-model="pullingModel"
              :deleting-model="deletingModel"
              :get-model-display-name="getModelDisplayName"
              :format-model-size="formatModelSize"
              :fetch-ollama-models="fetchOllamaModels"
              :pull-model="pullModel"
              :delete-model="deleteModel"
              :clear-models-error="clearModelsError"
              :on-select-model="handleSelectModel"
            />
          
          <!-- Audio Loopback Tab -->
          <AudioTab
            v-if="activeTab === 'audio'"
            :audio-devices="audioDevices"
            :is-loading-audio-devices="isLoadingAudioDevices"
            :audio-devices-error="audioDevicesError"
            :testing-device-id="testingDeviceId"
            :audio-settings="audioSettings"
            :enumerate-audio-devices="enumerateAudioDevices"
            :clear-audio-error="clearAudioError"
            :select-audio-device="selectAudioDevice"
            :get-device-icon="getDeviceIcon"
            :get-device-method-badge="getDeviceMethodBadge"
            :set-audio-loopback-enabled="setAudioLoopbackEnabled"
            :set-buffer-size="setBufferSize"
            :set-sample-rate="setSampleRate"
           />
           
           <!-- Documents Management Tab -->
          <DocumentsTab
            v-if="activeTab === 'documents'"
            :documents="ragDocuments.documents.value"
            :cached-documents="ragDocuments.cachedDocuments.value"
            :selected-ids="ragDocuments.selectedDocumentIds.value"
            :total-storage-size-m-b="ragDocuments.totalStorageSizeMB.value"
            :settings-max-doc-size-mb="ragDocuments.settings.value?.max_document_size_mb || 50"
            :max-cached-documents="ragDocuments.settings.value?.max_cached_documents || 5"
            :is-uploading="isUploading"
            :is-drag-over="isDragOver"
            :handle-file-upload="handleFileUpload"
            :handle-drag-over="handleDragOver"
            :handle-drag-leave="handleDragLeave"
            :handle-drop="handleDrop"
            :clear-all-selections="clearAllSelections"
            :select-all-documents="ragDocuments.selectAllDocuments"
            :toggle-document-selection="toggleDocumentSelection"
            :delete-document="deleteDocument"
            :generate-embeddings="generateEmbeddings"
            :get-storage-stats="ragDocuments.getStorageStats"
            :get-document-icon="getDocumentIcon"
            :format-file-size="formatFileSize"
            @clearCache="clearCache"
          />
          
          <!-- General Settings Tab -->
          <GeneralTab
            v-if="activeTab === 'general'"
            :general-settings="generalSettings"
            :transparency="transparency"
            :handle-transparency-level-change="handleTransparencyLevelChange"
            :fetch-system-info="fetchSystemInfo"
            :system-info="systemInfo"
            :is-loading-system-info="isLoadingSystemInfo"
            :system-info-error="systemInfoError"
            :format-gpu-memory="formatGpuMemory"
            :set-general-setting="(key: string, value: any) => { (generalSettings as any).value[key] = value }"
          />
        </div>
      </div>
    </div>
  </Transition>
</template>

<style>
/* Settings Drawer - Standalone Window */
.settings-drawer {
  @apply backdrop-blur-xl border border-white/15 rounded-2xl;
  background: linear-gradient(to bottom, 
    rgba(10, 10, 12, 0.9) 0%, 
    rgba(5, 5, 7, 0.95) 100%
  );
  width: 900px;
  height: 600px;
  max-width: 95vw;
  max-height: 95vh;
  display: flex;
  flex-direction: column;
  
  /* Enhanced glass effect similar to transparency controls */
  backdrop-filter: blur(80px) saturate(180%);
  box-shadow: 
    0 25px 80px rgba(0, 0, 0, 0.6),
    0 10px 30px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

/* Drawer Header */
.drawer-header {
  @apply flex items-center justify-between px-6 py-4 border-b border-white/10;
  flex-shrink: 0;
}

.drawer-title {
  @apply flex items-center gap-3;
}

.drawer-close-btn {
  @apply rounded-full p-2 hover:bg-white/10 transition-colors;
}

/* Drawer Body */
.drawer-body {
  @apply flex flex-1;
  min-height: 0;
}

/* Settings Navigation (Left Sidebar) */
.settings-nav {
  @apply border-r border-white/10;
  background: linear-gradient(135deg, 
    rgba(0, 0, 0, 0.6) 0%, 
    rgba(10, 10, 15, 0.5) 50%,
    rgba(0, 0, 0, 0.6) 100%
  );
  width: 300px;
  flex-shrink: 0;
  padding: 24px 0;
}

.nav-item {
  @apply flex items-center gap-4 px-6 py-4 text-left transition-all duration-300 relative;
  width: calc(100% - 24px);
  margin: 0 12px;
  border-radius: 16px;
  margin-bottom: 8px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
  overflow: hidden;
}

.nav-item::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
  opacity: 0;
  transition: opacity 0.3s ease;
}

.nav-item.nav-active {
  background: linear-gradient(135deg, 
    rgba(59, 130, 246, 0.2) 0%, 
    rgba(37, 99, 235, 0.15) 50%,
    rgba(59, 130, 246, 0.1) 100%
  );
  border-color: rgba(59, 130, 246, 0.3);
  box-shadow: 
    0 8px 32px rgba(59, 130, 246, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

.nav-item.nav-active::before {
  opacity: 1;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.15);
  transform: translateX(4px) translateY(-1px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
}

.nav-item:hover::before {
  opacity: 1;
}

.nav-icon {
  @apply text-white/70;
  flex-shrink: 0;
}

.nav-item.nav-active .nav-icon {
  @apply text-blue-400;
}

.nav-content {
  @apply flex-1;
}

.nav-title {
  @apply text-white/90 font-medium text-sm;
}

.nav-subtitle {
  @apply text-white/60 text-xs mt-0.5;
}

.nav-status {
  @apply flex items-center;
  flex-shrink: 0;
}

.status-dot {
  @apply w-2.5 h-2.5 rounded-full relative;
  filter: drop-shadow(0 0 4px currentColor);
}

.status-success .status-dot {
  @apply bg-green-400;
  animation: pulse-green 2s infinite;
}

.status-error .status-dot {
  @apply bg-red-400;
  animation: pulse-red 2s infinite;
}

.status-warning .status-dot {
  @apply bg-yellow-400;
  animation: pulse-yellow 2s infinite;
}

@keyframes pulse-green {
  0%, 100% { 
    opacity: 1; 
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.4);
  }
  50% { 
    opacity: 0.7; 
    box-shadow: 0 0 12px rgba(34, 197, 94, 0.6);
  }
}

@keyframes pulse-red {
  0%, 100% { 
    opacity: 1; 
    box-shadow: 0 0 8px rgba(248, 113, 113, 0.4);
  }
  50% { 
    opacity: 0.7; 
    box-shadow: 0 0 12px rgba(248, 113, 113, 0.6);
  }
}

@keyframes pulse-yellow {
  0%, 100% { 
    opacity: 1; 
    box-shadow: 0 0 8px rgba(251, 191, 36, 0.4);
  }
  50% { 
    opacity: 0.7; 
    box-shadow: 0 0 12px rgba(251, 191, 36, 0.6);
  }
}

/* Settings Content Area */
.settings-content {
  @apply flex-1 overflow-y-auto;
  min-height: 0;
}

.settings-section {
  @apply p-8;
  min-height: 100%;
}

.section-header {
  @apply mb-8;
}

.section-title {
  @apply text-2xl font-bold text-white mb-2;
}

.section-description {
  @apply text-white/70 text-sm leading-relaxed;
}

/* AI Models Styles (inherited from AIModelsPanel) */
.ollama-status {
  @apply mb-4;
}

.models-section {
  @apply mt-4;
}

.models-header {
  @apply flex items-center justify-between mb-3;
}

.refresh-btn {
  @apply p-1 rounded-lg transition-all duration-200 text-white/70 hover:text-white;
  background: rgba(255, 255, 255, 0.05);
}

.refresh-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  transform: rotate(90deg);
}

.error-message {
  @apply flex items-center justify-between bg-red-500/20 border border-red-400/30 rounded-lg p-2 mb-3;
}

.loading-state {
  @apply p-4 text-center;
}

.models-list {
  @apply space-y-3 mb-6 max-h-64 overflow-y-auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
}

.model-item {
  @apply flex items-center justify-between p-5 rounded-2xl border transition-all duration-300 relative overflow-hidden;
  background: linear-gradient(135deg, 
    rgba(255, 255, 255, 0.06) 0%, 
    rgba(255, 255, 255, 0.02) 50%,
    rgba(255, 255, 255, 0.04) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(20px);
}

.model-item::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.03) 0%, rgba(147, 51, 234, 0.02) 100%);
  opacity: 0;
  transition: opacity 0.3s ease;
}

.model-item:hover {
  background: linear-gradient(135deg, 
    rgba(255, 255, 255, 0.12) 0%, 
    rgba(255, 255, 255, 0.06) 50%,
    rgba(255, 255, 255, 0.08) 100%
  );
  border-color: rgba(255, 255, 255, 0.25);
  transform: translateY(-2px) scale(1.01);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.4),
    0 0 0 1px rgba(255, 255, 255, 0.1);
}

.model-item:hover::before {
  opacity: 1;
}

.model-info {
  @apply flex-1;
}

.model-name {
  @apply text-white/90 font-medium text-sm;
}

.model-details {
  @apply flex items-center gap-2 mt-1;
}

.model-size {
  @apply text-white/60 text-xs;
}

.model-params {
  @apply text-white/60 text-xs px-1.5 py-0.5 bg-white/10 rounded-md;
}

.model-actions {
  @apply flex items-center gap-2;
}

.select-btn {
  @apply w-8 h-8 rounded-full border-2 border-white/30 text-sm flex items-center justify-center transition-all duration-200;
  font-weight: bold;
  position: relative;
  z-index: 20;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.05);
}

.select-btn:hover:not(:disabled) {
  @apply border-white/50 transform scale-110;
  background: rgba(255, 255, 255, 0.15);
}

.select-btn:active:not(:disabled) {
  @apply transform scale-95;
}

.select-btn:disabled {
  @apply opacity-50 cursor-not-allowed;
}

.select-btn.active {
  @apply bg-green-500/80 border-green-400 text-white shadow-lg;
  box-shadow: 0 0 0 3px rgba(34, 197, 94, 0.2);
}

.select-btn.active:hover:not(:disabled) {
  @apply bg-green-500 border-green-300;
}

.delete-btn {
  @apply p-1.5 rounded-lg bg-red-500/20 hover:bg-red-500/40 transition-colors text-red-400 hover:text-red-300;
}

.delete-btn:disabled {
  @apply opacity-50 cursor-not-allowed;
}

.no-models {
  @apply text-center p-4 rounded-lg border border-white/10;
  background: rgba(255, 255, 255, 0.02);
}

.pull-model-section {
  @apply mt-4 pt-4 border-t border-white/10;
}

.popular-models {
  @apply grid grid-cols-2 gap-2;
}

.model-pull-btn {
  @apply flex items-center gap-2 p-3 rounded-xl border text-sm font-medium transition-all duration-300 relative overflow-hidden;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(37, 99, 235, 0.05) 100%);
  border: 1px solid rgba(59, 130, 246, 0.2);
  color: rgb(147, 197, 253);
}

.model-pull-btn::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(37, 99, 235, 0.05) 100%);
  opacity: 0;
  transition: opacity 0.3s ease;
}

.model-pull-btn:hover {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.25) 0%, rgba(37, 99, 235, 0.15) 100%);
  border-color: rgba(59, 130, 246, 0.5);
  color: rgb(191, 219, 254);
  transform: translateY(-2px) scale(1.02);
  box-shadow: 
    0 8px 24px rgba(59, 130, 246, 0.25),
    0 0 0 1px rgba(59, 130, 246, 0.1);
}

.model-pull-btn:hover::before {
  opacity: 1;
}

.model-pull-btn:disabled {
  @apply opacity-50 cursor-not-allowed;
}

.model-pull-btn.recommended {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(22, 163, 74, 0.1) 100%);
  border-color: rgba(34, 197, 94, 0.3);
  color: rgb(187, 247, 208);
}

.model-pull-btn.recommended:hover {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.3) 0%, rgba(22, 163, 74, 0.2) 100%);
  border-color: rgba(34, 197, 94, 0.6);
  color: rgb(220, 252, 231);
  box-shadow: 
    0 8px 24px rgba(34, 197, 94, 0.25),
    0 0 0 1px rgba(34, 197, 94, 0.1);
}

.recommended-badge {
  @apply text-xs px-2 py-1 rounded-lg font-bold;
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.9) 0%, rgba(22, 163, 74, 0.8) 100%);
  color: rgb(20, 83, 45);
  box-shadow: 0 2px 8px rgba(34, 197, 94, 0.3);
}

.vision-badge {
  @apply text-xs px-2 py-1 rounded-lg font-bold;
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.9) 0%, rgba(147, 51, 234, 0.8) 100%);
  color: rgb(76, 29, 149);
  box-shadow: 0 2px 8px rgba(168, 85, 247, 0.3);
}

.coding-badge {
  @apply text-xs px-2 py-1 rounded-lg font-bold;
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.9) 0%, rgba(22, 163, 74, 0.8) 100%);
  color: rgb(20, 83, 45);
  box-shadow: 0 2px 8px rgba(34, 197, 94, 0.3);
}

.research-badge {
  @apply text-xs px-2 py-1 rounded-lg font-bold;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.9) 0%, rgba(37, 99, 235, 0.8) 100%);
  color: rgb(30, 58, 138);
  box-shadow: 0 2px 8px rgba(59, 130, 246, 0.3);
}

.model-pull-btn.vision-model {
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.15) 0%, rgba(147, 51, 234, 0.1) 100%);
  border-color: rgba(168, 85, 247, 0.3);
  color: rgb(221, 179, 255);
}

.model-pull-btn.vision-model:hover {
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.3) 0%, rgba(147, 51, 234, 0.2) 100%);
  border-color: rgba(168, 85, 247, 0.6);
  color: rgb(237, 201, 255);
  box-shadow: 
    0 8px 24px rgba(168, 85, 247, 0.25),
    0 0 0 1px rgba(168, 85, 247, 0.1);
}

.model-pull-btn.coding-model {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(22, 163, 74, 0.1) 100%);
  border-color: rgba(34, 197, 94, 0.3);
  color: rgb(187, 247, 208);
}

.model-pull-btn.coding-model:hover {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.3) 0%, rgba(22, 163, 74, 0.2) 100%);
  border-color: rgba(34, 197, 94, 0.6);
  color: rgb(220, 252, 231);
  box-shadow: 
    0 8px 24px rgba(34, 197, 94, 0.25),
    0 0 0 1px rgba(34, 197, 94, 0.1);
}

.model-pull-btn.research-model {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.15) 0%, rgba(37, 99, 235, 0.1) 100%);
  border-color: rgba(59, 130, 246, 0.3);
  color: rgb(147, 197, 253);
}

.model-pull-btn.research-model:hover {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.3) 0%, rgba(37, 99, 235, 0.2) 100%);
  border-color: rgba(59, 130, 246, 0.6);
  color: rgb(191, 219, 254);
  box-shadow: 
    0 8px 24px rgba(59, 130, 246, 0.25),
    0 0 0 1px rgba(59, 130, 246, 0.1);
}

/* Audio Device Styles */
.audio-settings-header {
  @apply mb-4;
}

.audio-devices-list {
  @apply space-y-4 mb-6 max-h-64 overflow-y-auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
}

.audio-device-item {
  @apply flex items-center justify-between p-5 rounded-2xl border transition-all duration-300 relative overflow-hidden;
  background: linear-gradient(135deg, 
    rgba(255, 255, 255, 0.06) 0%, 
    rgba(255, 255, 255, 0.02) 50%,
    rgba(255, 255, 255, 0.04) 100%
  );
  border: 1px solid rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(20px);
}

.audio-device-item::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.03) 0%, rgba(236, 72, 153, 0.02) 100%);
  opacity: 0;
  transition: opacity 0.3s ease;
  pointer-events: none; /* Allow clicks to pass through */
  z-index: 1;
}

.audio-device-item:hover {
  background: linear-gradient(135deg, 
    rgba(255, 255, 255, 0.12) 0%, 
    rgba(255, 255, 255, 0.06) 50%,
    rgba(255, 255, 255, 0.08) 100%
  );
  border-color: rgba(255, 255, 255, 0.25);
  transform: translateY(-2px) scale(1.01);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.4),
    0 0 0 1px rgba(255, 255, 255, 0.1);
}

.audio-device-item:hover::before {
  opacity: 1;
}

.device-info {
  @apply flex-1;
  position: relative;
  z-index: 2;
}

.device-header {
  @apply flex items-center gap-2 mb-1;
}

.device-name {
  @apply text-white/90 font-medium text-sm;
}

.default-badge {
  @apply text-xs bg-yellow-400/80 text-yellow-900 px-1.5 py-0.5 rounded-md font-medium;
}

.device-details {
  @apply flex items-center gap-2 flex-wrap;
}

.device-spec {
  @apply text-white/60 text-xs;
}

.method-badge {
  @apply text-xs px-1.5 py-0.5 rounded-md font-medium border;
}

.device-actions {
  @apply flex items-center gap-2;
  position: relative;
  z-index: 10;
}

.no-devices {
  @apply text-center p-4 rounded-lg border border-white/10;
  background: rgba(255, 255, 255, 0.02);
}

.audio-buffer-settings {
  @apply mt-4 pt-4 border-t border-white/10;
}

/* Settings Form Styles */
.settings-group {
  @apply space-y-4;
}

.setting-item {
  @apply space-y-1;
}

.setting-label {
  @apply flex items-center gap-2 cursor-pointer;
}

.setting-label-full {
  @apply flex flex-col gap-2;
}

.setting-checkbox {
  @apply w-5 h-5 rounded-md border-white/20 text-blue-500 focus:ring-blue-500 focus:ring-offset-0 focus:ring-2;
  background: rgba(255, 255, 255, 0.05);
}

.setting-checkbox:checked {
  background: rgba(59, 130, 246, 0.8);
  border-color: rgba(59, 130, 246, 0.6);
}

.setting-select {
  @apply w-full px-4 py-3 border border-white/10 rounded-xl text-white/90 text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 backdrop-blur-sm transition-all duration-200;
  background: rgba(255, 255, 255, 0.05);
}

.setting-select:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.2);
}

.setting-select option {
  @apply bg-gray-800 text-white;
}

.setting-range {
  @apply w-full h-2 rounded-lg appearance-none cursor-pointer;
  background: rgba(255, 255, 255, 0.05);
}

.setting-range::-webkit-slider-thumb {
  @apply appearance-none w-4 h-4 bg-blue-500 rounded-full cursor-pointer;
}

.setting-range::-moz-range-thumb {
  @apply w-4 h-4 bg-blue-500 rounded-full cursor-pointer border-0;
}

/* Tab Description */
.tab-description {
  @apply mb-4 pb-4 border-b border-white/10;
}

/* Settings Separator */
.setting-separator {
  @apply my-4 border-t border-white/10;
}

/* Transparency Status Section */
.transparency-status-section {
  @apply mt-4;
}

.transparency-status-section .flex {
  @apply transition-all duration-200;
}

.transparency-status-section button {
  @apply transition-all duration-200;
}

.transparency-status-section button:hover {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

/* Transitions */
.settings-drawer-enter-active,
.settings-drawer-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.settings-drawer-enter-from {
  opacity: 0;
  transform: translateY(-10px);
}

.settings-drawer-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

/* Scrollbar Styles */
.models-list::-webkit-scrollbar,
.audio-devices-list::-webkit-scrollbar,
.panel-content::-webkit-scrollbar,
.tab-content::-webkit-scrollbar {
  width: 4px;
}

.models-list::-webkit-scrollbar-track,
.audio-devices-list::-webkit-scrollbar-track,
.panel-content::-webkit-scrollbar-track,
.tab-content::-webkit-scrollbar-track {
  background: transparent;
}

.models-list::-webkit-scrollbar-thumb,
.audio-devices-list::-webkit-scrollbar-thumb,
.panel-content::-webkit-scrollbar-thumb,
.tab-content::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

.tab-content {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
}

/* System Information Styles */
.system-info {
  @apply space-y-2;
}

.info-item {
  @apply flex items-center gap-2 text-sm;
}

.info-label {
  @apply text-white/60 font-medium min-w-[80px];
}

.info-value {
  @apply text-white/90;
}

.gpu-section {
  @apply mt-3 space-y-3;
}

.gpu-info {
  @apply bg-white/5 rounded-lg p-3 border border-white/10;
}

.gpu-header {
  @apply flex items-center gap-2 mb-2;
}

.gpu-name {
  @apply text-white/90 font-medium text-sm;
}

.gpu-vendor {
  @apply text-white/60 text-xs;
}

.gpu-details {
  @apply grid grid-cols-2 gap-2 text-xs;
}

.gpu-detail {
  @apply flex items-center gap-1;
}

.detail-label {
  @apply text-white/50;
}

.detail-value {
  @apply text-white/80;
}

.refresh-button {
  @apply flex items-center gap-2 px-3 py-1.5 bg-white/10 hover:bg-white/20 text-white/80 hover:text-white rounded-lg transition-colors text-sm;
}

/* Documents Management Styles */
.documents-upload-section {
  @apply mb-6;
}

.upload-header {
  @apply flex items-center justify-between mb-4;
}

.upload-actions {
  @apply flex items-center gap-3;
}

.upload-btn {
  @apply flex items-center gap-2 px-4 py-2 rounded-xl border text-sm font-medium transition-all duration-300 relative overflow-hidden;
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(22, 163, 74, 0.1) 100%);
  border: 1px solid rgba(34, 197, 94, 0.3);
  color: rgb(187, 247, 208);
}

.upload-btn:hover:not(:disabled) {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.3) 0%, rgba(22, 163, 74, 0.2) 100%);
  border-color: rgba(34, 197, 94, 0.6);
  color: rgb(220, 252, 231);
  transform: translateY(-2px) scale(1.02);
  box-shadow: 0 8px 24px rgba(34, 197, 94, 0.25);
}

.upload-btn:disabled {
  @apply opacity-50 cursor-not-allowed;
}

.upload-dropzone {
  @apply p-8 border-2 border-dashed rounded-2xl cursor-pointer transition-all duration-300;
  border-color: rgba(255, 255, 255, 0.2);
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.02) 0%, rgba(255, 255, 255, 0.01) 100%);
}

.upload-dropzone:hover {
  border-color: rgba(34, 197, 94, 0.4);
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.05) 0%, rgba(22, 163, 74, 0.03) 100%);
  transform: translateY(-2px);
}

.upload-dropzone.drag-over {
  border-color: rgba(34, 197, 94, 0.6);
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.1) 0%, rgba(22, 163, 74, 0.05) 100%);
}

.upload-dropzone.uploading {
  border-color: rgba(59, 130, 246, 0.4);
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.05) 0%, rgba(37, 99, 235, 0.03) 100%);
}

.dropzone-content {
  @apply text-center;
}

.upload-progress {
  @apply w-full;
}

.progress-bar {
  @apply w-full h-2 bg-white/10 rounded-full overflow-hidden;
}

.progress-fill {
  @apply h-full bg-gradient-to-r from-green-500 to-blue-500 rounded-full;
  animation: progress-pulse 2s ease-in-out infinite;
  width: 100%;
}

@keyframes progress-pulse {
  0%, 100% { transform: translateX(-100%); }
  50% { transform: translateX(0%); }
}

.storage-stats-section {
  @apply mb-6;
}

.stats-header {
  @apply flex items-center justify-between mb-4;
}

.stats-grid {
  @apply grid grid-cols-2 md:grid-cols-4 gap-4;
}

.stat-item {
  @apply bg-white/5 rounded-xl p-4 border border-white/10 text-center;
}

.stat-value {
  @apply text-2xl font-bold text-white/90 mb-1;
}

.stat-label {
  @apply text-xs text-white/60;
}

.documents-library-section {
  @apply mb-6;
}

.library-header {
  @apply flex items-center justify-between mb-4;
}

.library-actions {
  @apply flex items-center gap-2;
}

.action-btn {
  @apply px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200;
}

.action-btn.secondary {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.8);
}

.action-btn.secondary:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.2);
  color: rgba(255, 255, 255, 0.9);
}

.action-btn.small {
  @apply px-2 py-1 text-xs;
}

.action-btn.danger {
  background: rgba(239, 68, 68, 0.15);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: rgb(252, 165, 165);
}

.action-btn.danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.25);
  border-color: rgba(239, 68, 68, 0.5);
  color: rgb(254, 202, 202);
}

.action-btn:disabled {
  @apply opacity-50 cursor-not-allowed;
}

.documents-list {
  @apply space-y-3 max-h-96 overflow-y-auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
}

.document-item {
  @apply flex items-center gap-4 p-4 rounded-2xl border transition-all duration-300 relative overflow-hidden;
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.06) 0%, rgba(255, 255, 255, 0.02) 100%);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.document-item:hover {
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.12) 0%, rgba(255, 255, 255, 0.06) 100%);
  border-color: rgba(255, 255, 255, 0.25);
  transform: translateY(-1px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

.document-item.selected {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.15) 0%, rgba(22, 163, 74, 0.1) 100%);
  border-color: rgba(34, 197, 94, 0.3);
}

.document-item.cached {
  border-left: 3px solid #fbbf24;
}

.document-checkbox {
  @apply flex-shrink-0;
}

.document-icon {
  @apply text-2xl flex-shrink-0;
}

.document-info {
  @apply flex-1 min-w-0;
}

.document-name {
  @apply text-white/90 font-medium text-sm truncate;
}

.document-meta {
  @apply flex items-center gap-1 text-xs text-white/50 mt-1;
}

.separator {
  @apply text-white/20;
}

.document-status {
  @apply flex-shrink-0;
}

.cache-badge {
  @apply px-2 py-1 rounded-md text-xs font-medium;
}

.cache-badge.active {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.cache-badge:not(.active) {
  background: rgba(255, 255, 255, 0.05);
  color: rgba(255, 255, 255, 0.5);
}

.document-actions {
  @apply flex items-center gap-2 flex-shrink-0;
}

.empty-documents {
  @apply text-center py-12;
}

.cache-management-section {
  @apply mt-6 pt-6 border-t border-white/10;
}

.cache-header {
  @apply flex items-center justify-between mb-4;
}

.cache-info {
  @apply text-right;
}

.cache-actions {
  @apply mb-4;
}

.cache-documents {
  @apply space-y-2;
}

.cache-document-item {
  @apply flex items-center gap-3 p-3 rounded-lg;
  background: rgba(251, 191, 36, 0.1);
  border: 1px solid rgba(251, 191, 36, 0.2);
}

.cache-doc-icon {
  @apply text-lg flex-shrink-0;
}

.cache-doc-info {
  @apply flex-1;
}

.cache-doc-name {
  @apply text-white/90 text-sm font-medium;
}

.cache-doc-meta {
  @apply text-white/60 text-xs mt-0.5;
}

.cache-indicator {
  @apply text-yellow-400 flex-shrink-0;
}
</style>