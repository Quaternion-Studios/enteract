import { ref, computed, watch, onMounted, onUnmounted, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface TransparencyState {
  level: number
  isTransparent: boolean
  isClickThrough: boolean
  isVisible: boolean
}

export function useTransparency() {
  // Reactive state
  const transparencyLevel = ref<number>(1.0)  // 0.0 = invisible, 1.0 = solid
  const isEnabled = ref<boolean>(false)
  const isLoading = ref<boolean>(false)
  const lastError = ref<string | null>(null)
  
  // Computed properties
  const isTransparent = computed(() => transparencyLevel.value < 0.95)
  const isClickThrough = computed(() => transparencyLevel.value < 0.1)
  const isVisible = computed(() => transparencyLevel.value > 0.05)
  
  const transparencyState = computed<TransparencyState>(() => ({
    level: transparencyLevel.value,
    isTransparent: isTransparent.value,
    isClickThrough: isClickThrough.value,
    isVisible: isVisible.value
  }))

  // Apply transparency to OS window
  const applyTransparency = async (level: number): Promise<void> => {
    if (isLoading.value) return
    
    try {
      isLoading.value = true
      lastError.value = null
      
      const clampedLevel = Math.max(0.1, Math.min(1, level)) // Minimum 10% opacity to prevent full invisibility
      console.log(`🔧 TRANSPARENCY: Applying level ${clampedLevel} (original: ${level})`)
      
      await invoke('set_window_transparency', { alpha: clampedLevel })
      
      transparencyLevel.value = clampedLevel
      
      console.log(`✅ TRANSPARENCY: Successfully applied ${clampedLevel}`)
      
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : 'Failed to set transparency'
      console.error('🚨 TRANSPARENCY ERROR:', error)
      
      // Emergency restore on error
      try {
        console.log('🔄 TRANSPARENCY: Emergency restore due to error')
        await invoke('emergency_restore_window')
        transparencyLevel.value = 1.0
      } catch (restoreError) {
        console.error('🚨 EMERGENCY RESTORE FAILED:', restoreError)
      }
    } finally {
      isLoading.value = false
    }
  }

  // Toggle transparency on/off
  const toggle = async (): Promise<void> => {
    try {
      const result = await invoke<number>('toggle_transparency', { 
        currentAlpha: transparencyLevel.value 
      })
      transparencyLevel.value = result
      isEnabled.value = result < 1.0
      
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : 'Failed to toggle transparency'
      console.error('Toggle transparency error:', error)
    }
  }

  // Set specific transparency level
  const setLevel = async (level: number): Promise<void> => {
    await applyTransparency(level)
    isEnabled.value = level < 1.0
  }

  // Preset transparency levels (with safety minimums)
  const presets = {
    invisible: () => setLevel(0.1), // Changed from 0.0 to prevent full invisibility
    ghostMode: () => setLevel(0.3),
    semiTransparent: () => setLevel(0.7),
    solid: () => setLevel(1.0)
  }

  // Emergency restore - always works
  const emergencyRestore = async (): Promise<void> => {
    try {
      await invoke('emergency_restore_window')
      transparencyLevel.value = 1.0
      isEnabled.value = false
      
      // Clear error state
      lastError.value = null
      
    } catch (error) {
      console.error('Emergency restore failed:', error)
      // Even if the invoke fails, update local state
      transparencyLevel.value = 1.0
      isEnabled.value = false
    }
  }

  // Setup and cleanup
  onMounted(() => {
    // Temporarily disable loading preferences to debug window disappearing issue
    console.log('🔧 TRANSPARENCY DEBUG: Skipping preference loading to debug window disappearing')
    // loadPreferences()
    
    // Keyboard shortcuts disabled - now controlled through settings panel
    // document.addEventListener('keydown', handleKeyDown)
    
    // Setup emergency safety timer (auto-restore after 30 seconds of full invisibility)
    let invisibilityTimer: number | null = null
    
    watch(transparencyLevel, (newLevel) => {
      if (newLevel <= 0.05) {
        // Start timer for emergency restore 
        invisibilityTimer = window.setTimeout(() => {
          console.warn('Auto-restoring visibility after 30 seconds of invisibility')
          emergencyRestore()
        }, 30000)
      } else {
        // Clear timer if window becomes visible
        if (invisibilityTimer) {
          clearTimeout(invisibilityTimer)
          invisibilityTimer = null
        }
      }
    })
  })

  onUnmounted(() => {
    // Cleanup placeholder - keyboard listeners disabled (controlled through settings panel)
  })

  // Utility functions
  const getTransparencyPercentage = () => Math.round(transparencyLevel.value * 100)
  const getVisibilityStatus = () => {
    if (transparencyLevel.value >= 0.95) return 'Solid'
    if (transparencyLevel.value >= 0.7) return 'Semi-transparent'
    if (transparencyLevel.value >= 0.3) return 'Ghost mode'
    if (transparencyLevel.value > 0.05) return 'Nearly invisible'
    return 'Invisible'
  }

  return {
    // State
    transparencyLevel: readonly(transparencyLevel),
    isEnabled: readonly(isEnabled),
    isLoading: readonly(isLoading),
    lastError: readonly(lastError),
    transparencyState,
    
    // Computed
    isTransparent,
    isClickThrough,
    isVisible,
    
    // Actions
    toggle,
    setLevel,
    applyTransparency,
    emergencyRestore,
    
    // Presets
    presets,
    
    // Utilities
    getTransparencyPercentage,
    getVisibilityStatus
  }
} 