<script setup lang="ts">
import { ref, watch, onMounted, nextTick } from 'vue'
import {
  Cog6ToothIcon,
  XMarkIcon,
  ArrowsPointingOutIcon,
  TrashIcon,
  ArrowDownTrayIcon,
  SpeakerWaveIcon,
  MicrophoneIcon,
  ComputerDesktopIcon,
  CpuChipIcon
} from '@heroicons/vue/24/outline'
import { useAIModels } from '../../composables/useAIModels'
import { useTransparency } from '../../composables/useTransparency'
import { useWindowRegistration } from '../../composables/useWindowRegistry'
import { invoke } from '@tauri-apps/api/core'

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
const activeTab = ref<'models' | 'audio' | 'general'>('models')

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
            <div v-if="activeTab === 'models'" class="settings-section">
            <div class="section-header">
              <h2 class="section-title">AI Models</h2>
              <p class="section-description">
                Manage AI models for transcription analysis and intelligent responses. Enteract runs models locally for privacy and performance.
              </p>
            </div>
            
            <!-- Ollama Status -->
            <div class="ollama-status">
              <div v-if="ollamaStatus.status === 'running'" class="status-good">
                <span class="text-green-400">● Model manager is running</span>
                <span v-if="ollamaStatus.version" class="text-white/60 text-xs ml-2">v{{ ollamaStatus.version }}</span>
              </div>
              <div v-else-if="ollamaStatus.status === 'not_running'" class="status-error">
                <span class="text-red-400">● Model manager is not running</span>
                <p class="text-white/60 text-xs mt-1">Please start Model manager to manage models</p>
              </div>
              <div v-else-if="ollamaStatus.status === 'checking'" class="status-loading">
                <span class="text-yellow-400">● Checking model manager status...</span>
              </div>
              <div v-else class="status-error">
                <span class="text-red-400">● Failed to connect to model manager</span>
              </div>
            </div>
            
            <!-- Models Management -->
            <div v-if="ollamaStatus.status === 'running'" class="models-section">
              <div class="models-header">
                <h3 class="text-white/90 font-medium">Available Models</h3>
                <button 
                  @click="() => fetchOllamaModels(true)" 
                  :disabled="isLoadingModels"
                  class="refresh-btn"
                  title="Refresh Models"
                >
                  <ArrowsPointingOutIcon class="w-4 h-4" :class="{ 'animate-spin': isLoadingModels }" />
                </button>
              </div>
              
              <!-- Error Message -->
              <div v-if="modelsError" class="error-message">
                <span class="text-red-400 text-sm">{{ modelsError }}</span>
                <button @click="clearModelsError" class="ml-2 text-white/60 hover:text-white">×</button>
              </div>
              
              <!-- Loading State -->
              <div v-if="isLoadingModels" class="loading-state">
                <div class="animate-pulse text-white/60">Loading models...</div>
              </div>
              
              <!-- Models List -->
              <div v-else-if="ollamaModels.length > 0" class="models-list">
                <div v-for="model in ollamaModels" :key="model.name" class="model-item">
                  <div class="model-info">
                    <div class="model-name">{{ getModelDisplayName(model) }}</div>
                    <div class="model-details">
                      <span class="model-size">{{ formatModelSize(model.size) }}</span>
                      <span v-if="model.details?.parameter_size" class="model-params">
                        {{ model.details.parameter_size }}
                      </span>
                    </div>
                  </div>
                  
                  <div class="model-actions">
                    <button
                      @click="selectedModel = model.name"
                      :class="{ 'active': selectedModel === model.name }"
                      class="select-btn"
                      title="Select Model"
                    >
                      {{ selectedModel === model.name ? '✓' : '○' }}
                    </button>
                    
                    <button
                      @click="deleteModel(model.name)"
                      :disabled="deletingModel === model.name"
                      class="delete-btn"
                      title="Delete Model"
                    >
                      <TrashIcon v-if="deletingModel !== model.name" class="w-3 h-3" />
                      <div v-else class="w-3 h-3 animate-spin">⟳</div>
                    </button>
                  </div>
                </div>
              </div>
              
              <!-- No Models -->
              <div v-else class="no-models">
                <p class="text-white/60 text-sm">No models available</p>
                <p class="text-white/40 text-xs mt-1">Pull a model to get started</p>
              </div>
              
              <!-- Pull Model Section -->
              <div class="pull-model-section">
                <h4 class="text-white/80 text-sm font-medium mb-2">Pull New Model</h4>
                <div class="popular-models">
                  <button 
                    v-for="modelName in ['gemma3:1b-it-qat', 'qwen2.5vl:3b', 'qwen2.5-coder:1.5b', 'deepseek-r1:1.5b', 'llama3.2']" 
                    :key="modelName"
                    @click="pullModel(modelName)"
                    :disabled="pullingModel === modelName"
                    class="model-pull-btn"
                    :class="{ 
                      'recommended': modelName === 'gemma3:1b-it-qat',
                      'vision-model': modelName === 'qwen2.5vl:3b',
                      'coding-model': modelName === 'qwen2.5-coder:1.5b',
                      'research-model': modelName === 'deepseek-r1:1.5b'
                    }"
                  >
                    <ArrowDownTrayIcon v-if="pullingModel !== modelName" class="w-3 h-3" />
                    <div v-else class="w-3 h-3 animate-spin">⟳</div>
                    <span>{{ modelName }}</span>
                    <span v-if="modelName === 'gemma3:1b-it-qat'" class="recommended-badge">Enteract Agent</span>
                    <span v-if="modelName === 'qwen2.5vl:3b'" class="vision-badge">Vision</span>
                    <span v-if="modelName === 'qwen2.5-coder:1.5b'" class="coding-badge">Coding</span>
                    <span v-if="modelName === 'deepseek-r1:1.5b'" class="research-badge">Research</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Audio Loopback Tab -->
          <div v-if="activeTab === 'audio'" class="settings-section">
            <div class="section-header">
              <h2 class="section-title">Audio Loopback</h2>
              <p class="section-description">
                Configure audio loopback to capture system audio for transcription. Perfect for meetings, videos, and any audio playing on your computer.
              </p>
            </div>
            
            <!-- Audio Settings Header -->
            <div class="audio-settings-header">
              <div class="flex items-center justify-between mb-4">
                <h3 class="text-white/90 font-medium">Audio Loopback Devices</h3>
                <button 
                  @click="enumerateAudioDevices" 
                  :disabled="isLoadingAudioDevices"
                  class="refresh-btn"
                  title="Refresh Audio Devices"
                >
                  <ArrowsPointingOutIcon class="w-4 h-4" :class="{ 'animate-spin': isLoadingAudioDevices }" />
                </button>
              </div>
              
              <!-- Loopback Enable Toggle -->
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="audioSettings.loopbackEnabled"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Enable Audio Loopback</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Capture system audio for conversational interface</p>
              </div>
            </div>
            
            <!-- Error Message -->
            <div v-if="audioDevicesError" class="error-message">
              <span class="text-red-400 text-sm">{{ audioDevicesError }}</span>
              <button @click="clearAudioError" class="ml-2 text-white/60 hover:text-white">×</button>
            </div>
            
            <!-- Loading State -->
            <div v-if="isLoadingAudioDevices" class="loading-state">
              <div class="animate-pulse text-white/60">Scanning WASAPI audio devices...</div>
            </div>
            
            <!-- Audio Devices List -->
            <div v-else-if="audioDevices.length > 0" class="audio-devices-list">
              <div v-for="device in audioDevices" :key="device.id" class="audio-device-item">
                <div class="device-info">
                  <div class="device-header">
                    <component :is="getDeviceIcon(device)" class="w-4 h-4 text-white/80" />
                    <div class="device-name">{{ device.name }}</div>
                    <div v-if="device.is_default" class="default-badge">Default</div>
                  </div>
                  
                  <div class="device-details">
                    <span class="device-spec">{{ device.sample_rate }} Hz</span>
                    <span class="device-spec">{{ device.channels }} ch</span>
                    <span class="device-spec">{{ device.format }}</span>
                    
                    <span 
                      class="method-badge"
                      :class="getDeviceMethodBadge(device.loopback_method).class"
                    >
                      {{ getDeviceMethodBadge(device.loopback_method).text }}
                    </span>
                  </div>
                </div>
                
                <div class="device-actions">
                  <button
                    @click="() => { console.log('Button clicked!', device.id); selectAudioDevice(device.id) }"
                    :class="{ 'active': audioSettings.selectedLoopbackDevice === device.id }"
                    class="select-btn"
                    title="Select Device"
                    :disabled="isLoadingAudioDevices || testingDeviceId !== null"
                    type="button"
                  >
                    <span v-if="testingDeviceId === device.id" class="animate-spin">⟳</span>
                    <span v-else>{{ audioSettings.selectedLoopbackDevice === device.id ? '✓' : '○' }}</span>
                  </button>
                </div>
              </div>
            </div>
            
            <!-- No Audio Devices -->
            <div v-else class="no-devices">
              <p class="text-white/60 text-sm">No loopback devices found</p>
              <p class="text-white/40 text-xs mt-1">Try enabling Stereo Mix in Windows Sound settings</p>
            </div>
            
            <!-- Audio Buffer Settings -->
            <div v-if="audioSettings.loopbackEnabled" class="audio-buffer-settings">
              <h4 class="text-white/80 text-sm font-medium mb-3">Buffer Settings</h4>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">Buffer Size: {{ audioSettings.bufferSize }} samples</span>
                  <input 
                    type="range" 
                    v-model.number="audioSettings.bufferSize"
                    min="1024"
                    max="8192"
                    step="1024"
                    class="setting-range"
                  >
                </label>
              </div>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">Sample Rate: {{ audioSettings.sampleRate }} Hz</span>
                  <select v-model.number="audioSettings.sampleRate" class="setting-select">
                    <option :value="16000">16000 Hz (Whisper)</option>
                    <option :value="44100">44100 Hz (CD Quality)</option>
                    <option :value="48000">48000 Hz (Studio)</option>
                  </select>
                </label>
              </div>
            </div>
          </div>
          
          <!-- General Settings Tab -->
          <div v-if="activeTab === 'general'" class="settings-section">
            <div class="section-header">
              <h2 class="section-title">General Settings</h2>
              <p class="section-description">
                Customize Enteract's behavior and appearance to match your workflow preferences.
              </p>
            </div>
            
            <div class="settings-group">
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.autoStartOllama"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Auto-start Ollama</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Automatically start Ollama when the app launches</p>
              </div>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.enableNotifications"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Enable Notifications</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Show desktop notifications for important events</p>
              </div>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">Theme</span>
                  <select v-model="generalSettings.theme" class="setting-select">
                    <option value="dark">Dark</option>
                    <option value="light">Light</option>
                    <option value="auto">Auto</option>
                  </select>
                </label>
              </div>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">Log Level</span>
                  <select v-model="generalSettings.logLevel" class="setting-select">
                    <option value="debug">Debug</option>
                    <option value="info">Info</option>
                    <option value="warn">Warning</option>
                    <option value="error">Error</option>
                  </select>
                </label>
              </div>
              
              <div class="setting-separator"></div>
              
              <h4 class="text-white/80 text-sm font-medium mb-3">Startup & Window</h4>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.startWithSystem"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Start with System</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Launch Enteract when your computer starts</p>
              </div>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.startMinimized"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Start Minimized</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Start in the system tray instead of showing the window</p>
              </div>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.saveWindowPosition"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Remember Window Position</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Restore window size and position on startup</p>
              </div>
              
              <div class="setting-separator"></div>
              
              <h4 class="text-white/80 text-sm font-medium mb-3">Transcription</h4>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">Default Language</span>
                  <select v-model="generalSettings.transcriptionLanguage" class="setting-select">
                    <option value="en">English</option>
                    <option value="es">Spanish</option>
                    <option value="fr">French</option>
                    <option value="de">German</option>
                    <option value="it">Italian</option>
                    <option value="pt">Portuguese</option>
                    <option value="ru">Russian</option>
                    <option value="ja">Japanese</option>
                    <option value="ko">Korean</option>
                    <option value="zh">Chinese</option>
                  </select>
                </label>
              </div>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">Microphone Whisper Model</span>
                  <select v-model="generalSettings.microphoneWhisperModel" class="setting-select">
                    <option value="tiny">Tiny (Fastest, Good accuracy)</option>
                    <option value="base">Base (Balanced speed/accuracy)</option>
                    <option value="small">Small (Best accuracy, Slower)</option>
                  </select>
                </label>
                <p class="text-white/60 text-xs mt-1">Model used for microphone transcription. Tiny is recommended for real-time performance.</p>
              </div>
              
              <div class="setting-item">
                <label class="setting-label-full">
                  <span class="text-white/90">System Audio Whisper Model</span>
                  <select v-model="generalSettings.loopbackWhisperModel" class="setting-select">
                    <option value="tiny">Tiny (Fastest, Good accuracy)</option>
                    <option value="base">Base (Balanced speed/accuracy)</option>
                    <option value="small">Small (Best accuracy, Slower)</option>
                  </select>
                </label>
                <p class="text-white/60 text-xs mt-1">Model used for system audio loopback transcription. Base is recommended for better accuracy with recorded audio.</p>
              </div>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.enableKeyboardShortcuts"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Enable Keyboard Shortcuts</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Use global keyboard shortcuts for quick actions</p>
              </div>
              
              <div class="setting-separator"></div>
              
              <h4 class="text-white/80 text-sm font-medium mb-3">Auto-save</h4>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.enableAutoSave"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Enable Auto-save</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Automatically save chat sessions</p>
              </div>
              
              <div class="setting-item" v-if="generalSettings.enableAutoSave">
                <label class="setting-label-full">
                  <span class="text-white/90">Auto-save Interval: {{ generalSettings.autoSaveInterval }} minutes</span>
                  <input 
                    type="range" 
                    v-model.number="generalSettings.autoSaveInterval"
                    min="1"
                    max="30"
                    step="1"
                    class="setting-range"
                  >
                </label>
              </div>
              
              <div class="setting-separator"></div>
              
              <h4 class="text-white/80 text-sm font-medium mb-3">Transparency</h4>
              
              <div class="setting-item">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.enableTransparency"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Enable Transparency</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Allow window transparency effects</p>
              </div>
              
              <div class="setting-item" v-if="generalSettings.enableTransparency">
                <label class="setting-label-full">
                  <span class="text-white/90">Default Transparency Level: {{ Math.round(generalSettings.defaultTransparencyLevel * 100) }}%</span>
                  <input 
                    type="range" 
                    v-model.number="generalSettings.defaultTransparencyLevel"
                    min="0.1"
                    max="1.0"
                    step="0.1"
                    class="setting-range"
                    @input="handleTransparencyLevelChange"
                  >
                </label>
                <p class="text-white/60 text-xs mt-1">Default opacity when transparency is enabled</p>
              </div>
              

              
              <div class="setting-item" v-if="generalSettings.enableTransparency">
                <label class="setting-label">
                  <input 
                    type="checkbox" 
                    v-model="generalSettings.autoRestoreOnError"
                    class="setting-checkbox"
                  >
                  <span class="text-white/90">Auto-restore on Error</span>
                </label>
                <p class="text-white/60 text-xs mt-1">Automatically restore window if transparency fails</p>
              </div>
              
              <!-- Current Transparency Status -->
              <div v-if="generalSettings.enableTransparency" class="transparency-status-section">
                <div class="flex items-center justify-between p-3 rounded-lg bg-white/5 border border-white/10">
                  <div class="flex items-center gap-2">
                    <div class="w-2 h-2 rounded-full" :class="{
                      'bg-green-400': transparency.isVisible.value && !transparency.isClickThrough.value,
                      'bg-yellow-400': transparency.isClickThrough.value,
                      'bg-red-400': !transparency.isVisible.value
                    }"></div>
                    <span class="text-white/80 text-sm">Current Status: {{ transparency.getVisibilityStatus() }}</span>
                  </div>
                  <div class="text-white/60 text-xs">
                    {{ transparency.getTransparencyPercentage() }}% opacity
                  </div>
                </div>
                
                <!-- Quick Transparency Controls -->
                <div class="flex gap-2 mt-3">
                  <button
                    @click="() => { console.log('🔧 Solid button clicked'); generalSettings.defaultTransparencyLevel = 1.0; handleTransparencyLevelChange(); }"
                    class="px-3 py-1.5 text-xs rounded-lg bg-white/10 hover:bg-white/20 text-white/80 hover:text-white transition-colors"
                  >
                    Solid
                  </button>
                  <button
                    @click="() => { console.log('🔧 Semi button clicked'); generalSettings.defaultTransparencyLevel = 0.7; handleTransparencyLevelChange(); }"
                    class="px-3 py-1.5 text-xs rounded-lg bg-white/10 hover:bg-white/20 text-white/80 hover:text-white transition-colors"
                  >
                    Semi
                  </button>
                  <button
                    @click="() => { console.log('🔧 Ghost button clicked'); generalSettings.defaultTransparencyLevel = 0.3; handleTransparencyLevelChange(); }"
                    class="px-3 py-1.5 text-xs rounded-lg bg-white/10 hover:bg-white/20 text-white/80 hover:text-white transition-colors"
                  >
                    Ghost
                  </button>
                  <button
                    @click="() => { console.log('🔧 Restore button clicked'); generalSettings.defaultTransparencyLevel = 1.0; handleTransparencyLevelChange(); }"
                    class="px-3 py-1.5 text-xs rounded-lg bg-red-500/20 hover:bg-red-500/30 text-red-400 hover:text-red-300 transition-colors"
                  >
                    Restore
                  </button>
                </div>
              </div>
              
              <div class="setting-separator"></div>
              
              <!-- System Information -->
              <h4 class="text-white/80 text-sm font-medium mb-3">System Information</h4>
              
              <div v-if="isLoadingSystemInfo" class="text-white/60 text-sm">
                Loading system information...
              </div>
              
              <div v-else-if="systemInfoError" class="text-red-400 text-sm">
                Failed to load system info: {{ systemInfoError }}
              </div>
              
              <div v-else-if="systemInfo" class="system-info">
                <div class="info-item">
                  <span class="info-label">OS:</span>
                  <span class="info-value">{{ systemInfo.os }}</span>
                </div>
                
                <div class="info-item">
                  <span class="info-label">CPU:</span>
                  <span class="info-value">{{ systemInfo.cpu_name }}</span>
                </div>
                
                <div class="info-item">
                  <span class="info-label">Memory:</span>
                  <span class="info-value">{{ systemInfo.memory_gb.toFixed(1) }} GB</span>
                </div>
                
                <div v-if="systemInfo.gpus.length > 0" class="gpu-section">
                  <div class="info-item">
                    <span class="info-label">GPU{{ systemInfo.gpus.length > 1 ? 's' : '' }}:</span>
                  </div>
                  
                  <div v-for="(gpu, index) in systemInfo.gpus" :key="index" class="gpu-info">
                    <div class="gpu-header">
                      <CpuChipIcon class="w-4 h-4 text-white/60" />
                      <span class="gpu-name">{{ gpu.name }}</span>
                      <span class="gpu-vendor">({{ gpu.vendor }})</span>
                    </div>
                    
                    <div class="gpu-details">
                      <div v-if="gpu.memory_mb" class="gpu-detail">
                        <span class="detail-label">Memory:</span>
                        <span class="detail-value">{{ formatGpuMemory(gpu.memory_mb) }}</span>
                      </div>
                      
                      <div v-if="gpu.driver_version" class="gpu-detail">
                        <span class="detail-label">Driver:</span>
                        <span class="detail-value">{{ gpu.driver_version }}</span>
                      </div>
                      
                      <div v-if="gpu.temperature_celsius !== undefined" class="gpu-detail">
                        <span class="detail-label">Temperature:</span>
                        <span class="detail-value">{{ gpu.temperature_celsius }}°C</span>
                      </div>
                      
                      <div v-if="gpu.utilization_percent !== undefined" class="gpu-detail">
                        <span class="detail-label">Usage:</span>
                        <span class="detail-value">{{ gpu.utilization_percent }}%</span>
                      </div>
                    </div>
                  </div>
                </div>
                
                <div v-else class="info-item">
                  <span class="info-label">GPU:</span>
                  <span class="info-value text-white/60">No dedicated GPU detected</span>
                </div>
                
                <button @click="fetchSystemInfo" class="refresh-button mt-3">
                  <ArrowsPointingOutIcon class="w-4 h-4" />
                  Refresh System Info
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
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
</style>