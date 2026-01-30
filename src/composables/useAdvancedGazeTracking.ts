import { ref, computed, onUnmounted, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Interface definitions for enhanced gaze tracking
export interface MonitorInfo {
  x: number
  y: number
  width: number
  height: number
  is_primary: boolean
  name: string
  scale_factor: number
}

export interface MonitorMesh {
  monitors: MonitorInfo[]
  virtual_width: number
  virtual_height: number
  virtual_left: number
  virtual_top: number
  primary_monitor?: MonitorInfo
}

export interface AdvancedGazeData {
  x: number
  y: number
  confidence: number
  left_eye_landmarks: number[][]
  right_eye_landmarks: number[][]
  head_pose: {
    yaw: number
    pitch: number
    roll: number
  }
  timestamp: number
  monitor?: MonitorInfo
}

export interface GazeCalibrationPoint {
  screen_x: number
  screen_y: number
  gaze_x: number
  gaze_y: number
  confidence: number
  timestamp: number
}

export interface GazeTrackingConfig {
  camera_id: number
  screen_width: number
  screen_height: number
  smoothing_window: number
  confidence_threshold: number
  enable_monitor_mesh: boolean
  enable_advanced_calibration: boolean
  adaptive_smoothing: boolean
}

export interface GazeTrackingStats {
  total_frames_processed: number
  average_confidence: number
  frames_per_second: number
  tracking_duration: number
  last_update: number
  monitor_switches: number
  calibration_quality: number
}

export interface CalibrationSession {
  is_active: boolean
  current_point: number
  total_points: number
  collected_points: GazeCalibrationPoint[]
  target_positions: { x: number, y: number }[]
  quality_score: number
}

export function useAdvancedGazeTracking() {
  // Core state
  const isActive = ref(false)
  const isInitialized = ref(false)
  const isCalibrated = ref(false)
  const currentGaze = ref<AdvancedGazeData | null>(null)
  const error = ref<string | null>(null)
  const isLoading = ref(false)

  // Monitor and spatial awareness
  const monitorMesh = ref<MonitorMesh | null>(null)
  const currentMonitor = ref<MonitorInfo | null>(null)
  const gazeScreenPosition = ref<{ x: number, y: number } | null>(null)

  // Configuration
  const config = ref<GazeTrackingConfig>({
    camera_id: 0,
    screen_width: window.screen.width,
    screen_height: window.screen.height,
    smoothing_window: 8,
    confidence_threshold: 0.7,
    enable_monitor_mesh: true,
    enable_advanced_calibration: true,
    adaptive_smoothing: true
  })

  // Performance and statistics
  const stats = ref<GazeTrackingStats>({
    total_frames_processed: 0,
    average_confidence: 0,
    frames_per_second: 0,
    tracking_duration: 0,
    last_update: 0,
    monitor_switches: 0,
    calibration_quality: 0
  })

  // Calibration state
  const calibrationSession = ref<CalibrationSession>({
    is_active: false,
    current_point: 0,
    total_points: 9, // 3x3 grid by default
    collected_points: [],
    target_positions: [],
    quality_score: 0
  })

  // Tracking management
  let trackingInterval: number | null = null
  let statsInterval: number | null = null

  // Computed properties
  const isHighConfidence = computed(() => {
    return currentGaze.value ? currentGaze.value.confidence > config.value.confidence_threshold : false
  })

  const normalizedGaze = computed(() => {
    if (!currentGaze.value || !monitorMesh.value) return null
    
    return {
      x: (currentGaze.value.x - monitorMesh.value.virtual_left) / monitorMesh.value.virtual_width,
      y: (currentGaze.value.y - monitorMesh.value.virtual_top) / monitorMesh.value.virtual_height
    }
  })

  const trackingQuality = computed(() => {
    if (!currentGaze.value) return 'inactive'
    
    const confidence = currentGaze.value.confidence
    if (confidence > 0.9) return 'excellent'
    if (confidence > 0.8) return 'good'
    if (confidence > 0.6) return 'fair'
    if (confidence > 0.4) return 'poor'
    return 'very-poor'
  })

  const calibrationProgress = computed(() => {
    if (!calibrationSession.value.is_active) return 0
    return (calibrationSession.value.current_point / calibrationSession.value.total_points) * 100
  })

  // Initialize the enhanced gaze tracking system
  const initialize = async (): Promise<boolean> => {
    try {
      isLoading.value = true
      error.value = null

      console.log('🚀 Initializing Advanced Gaze Tracking System')

      // Start the enhanced ML eye tracking with new script
      const result = await invoke('start_ml_eye_tracking', { config: config.value })
      
      if (typeof result === 'string' && result.includes('success')) {
        isInitialized.value = true
        console.log('✅ Advanced gaze tracking initialized')
        
        // Load monitor mesh information
        await loadMonitorMesh()
        
        return true
      } else {
        error.value = 'Failed to initialize gaze tracking system'
        return false
      }

    } catch (err) {
      error.value = `Initialization error: ${(err as Error).message}`
      console.error('❌ Gaze tracking initialization failed:', err)
      return false
    } finally {
      isLoading.value = false
    }
  }

  // Load monitor mesh information from the Python script
  const loadMonitorMesh = async (): Promise<void> => {
    try {
      // The Python script outputs monitor information on startup
      // We'll get this through the stats or a dedicated call
      console.log('🖥️ Loading monitor mesh information...')
      
      // For now, create a fallback monitor mesh based on current screen
      const fallbackMesh: MonitorMesh = {
        monitors: [{
          x: 0,
          y: 0,
          width: window.screen.width,
          height: window.screen.height,
          is_primary: true,
          name: 'Primary_Monitor',
          scale_factor: window.devicePixelRatio || 1.0
        }],
        virtual_width: window.screen.width,
        virtual_height: window.screen.height,
        virtual_left: 0,
        virtual_top: 0
      }
      
      monitorMesh.value = fallbackMesh
      currentMonitor.value = fallbackMesh.monitors[0]
      
      console.log('🖥️ Monitor mesh loaded:', monitorMesh.value)
      
    } catch (err) {
      console.error('❌ Failed to load monitor mesh:', err)
    }
  }

  // Start gaze tracking
  const startTracking = async (): Promise<boolean> => {
    try {
      if (!isInitialized.value) {
        const initialized = await initialize()
        if (!initialized) return false
      }

      console.log('▶️ Starting gaze tracking...')
      
      // Start data polling
      startDataPolling()
      startStatsMonitoring()
      
      isActive.value = true
      stats.value.tracking_duration = Date.now()
      
      console.log('✅ Gaze tracking started successfully')
      return true

    } catch (err) {
      error.value = `Failed to start tracking: ${(err as Error).message}`
      console.error('❌ Failed to start gaze tracking:', err)
      return false
    }
  }

  // Stop gaze tracking
  const stopTracking = async (): Promise<void> => {
    try {
      if (!isActive.value) return
      
      console.log('⏹️ Stopping gaze tracking...')
      
      // Stop polling
      stopDataPolling()
      stopStatsMonitoring()

      // Stop the backend process
      await invoke('stop_ml_eye_tracking')
      
      isActive.value = false
      isInitialized.value = false
      currentGaze.value = null
      
      console.log('🛑 Gaze tracking stopped')

    } catch (err) {
      error.value = `Failed to stop tracking: ${(err as Error).message}`
      console.error('❌ Failed to stop gaze tracking:', err)
    }
  }

  // Start data polling loop
  const startDataPolling = (): void => {
    if (trackingInterval) return // Already polling

    console.log('📊 Starting data polling...')
    
    trackingInterval = window.setInterval(async () => {
      try {
        if (!isActive.value) return

        const gazeData = await invoke<AdvancedGazeData | null>('get_ml_gaze_data')
        
        if (gazeData) {
          currentGaze.value = gazeData
          updateGazeMetrics(gazeData)
          updateCurrentMonitor(gazeData)
        }
      } catch (err) {
        console.error('❌ Error polling for gaze data:', err)
        error.value = `Gaze polling error: ${(err as Error).message}`
        // Consider stopping polling on repeated errors
      }
    }, 33) // ~30 FPS
  }

  // Stop data polling
  const stopDataPolling = (): void => {
    if (trackingInterval) {
      clearInterval(trackingInterval)
      trackingInterval = null
      console.log('📊 Stopped data polling')
    }
  }

  // Start statistics monitoring
  const startStatsMonitoring = (): void => {
    if (statsInterval) return

    console.log('📊 Starting statistics monitoring...')
    
    statsInterval = window.setInterval(async () => {
      try {
        const trackingStats = await invoke('get_ml_tracking_stats') as any
        if (trackingStats) {
          updateStats(trackingStats)
        }
      } catch (err) {
        console.error('Stats monitoring error:', err)
      }
    }, 1000) // 1 second intervals
  }

  // Stop statistics monitoring
  const stopStatsMonitoring = (): void => {
    if (statsInterval) {
      clearInterval(statsInterval)
      statsInterval = null
      console.log('📊 Stopped statistics monitoring')
    }
  }

  // Update gaze metrics and screen position
  const updateGazeMetrics = (gaze: AdvancedGazeData): void => {
    // Update screen position
    gazeScreenPosition.value = { x: gaze.x, y: gaze.y }
    
    // Update rolling average confidence
    stats.value.average_confidence = (stats.value.average_confidence * 0.95) + (gaze.confidence * 0.05)
  }

  // Update current monitor based on gaze position
  const updateCurrentMonitor = (gaze: AdvancedGazeData): void => {
    if (!monitorMesh.value) return
    
    const previousMonitor = currentMonitor.value
    const newMonitor = monitorMesh.value.monitors.find(monitor => 
      gaze.x >= monitor.x && gaze.x < monitor.x + monitor.width &&
      gaze.y >= monitor.y && gaze.y < monitor.y + monitor.height
    )
    
    if (newMonitor && newMonitor !== previousMonitor) {
      currentMonitor.value = newMonitor
      stats.value.monitor_switches++
      console.log(`🖥️ Switched to monitor: ${newMonitor.name}`)
    }
  }

  // Update statistics
  const updateStats = (trackingStats: any): void => {
    if (trackingStats.frames_per_second) {
      stats.value.frames_per_second = trackingStats.frames_per_second
    }
    
    if (stats.value.tracking_duration > 0) {
      stats.value.tracking_duration = (Date.now() - stats.value.tracking_duration) / 1000
    }
  }

  // Advanced 9-point calibration with monitor awareness
  const startAdvancedCalibration = async (): Promise<boolean> => {
    try {
      console.log('🎯 Starting advanced calibration...')
      
      if (!monitorMesh.value) {
        error.value = 'Monitor mesh not available for calibration'
        return false
      }

      // Create calibration targets across all monitors
      const targets = generateCalibrationTargets()
      
      calibrationSession.value = {
        is_active: true,
        current_point: 0,
        total_points: targets.length,
        collected_points: [],
        target_positions: targets,
        quality_score: 0
      }

      // Start calibration mode in backend
      await invoke('calibrate_ml_eye_tracking')
      
      return true

    } catch (err) {
      error.value = `Calibration start error: ${(err as Error).message}`
      console.error('❌ Failed to start calibration:', err)
      return false
    }
  }

  // Generate calibration targets across monitor mesh
  const generateCalibrationTargets = (): { x: number, y: number }[] => {
    const targets: { x: number, y: number }[] = []
    
    if (!monitorMesh.value) return targets

    // For each monitor, add calibration points
    monitorMesh.value.monitors.forEach(monitor => {
      const margin = 100 // 100px margin from edges
      
      // 9-point grid per monitor
      for (let row = 0; row < 3; row++) {
        for (let col = 0; col < 3; col++) {
          const x = monitor.x + margin + (col * (monitor.width - 2 * margin)) / 2
          const y = monitor.y + margin + (row * (monitor.height - 2 * margin)) / 2
          targets.push({ x, y })
        }
      }
    })

    return targets
  }

  // Add calibration point during calibration session
  const addCalibrationPoint = async (screenX: number, screenY: number): Promise<boolean> => {
    try {
      if (!calibrationSession.value.is_active) return false

      // Add point to backend
      const success = await invoke('add_calibration_point', { 
        screen_x: screenX, 
        screen_y: screenY 
      })

      if (success) {
        calibrationSession.value.current_point++
        
        // If this was the last point, finish calibration
        if (calibrationSession.value.current_point >= calibrationSession.value.total_points) {
          return await finishCalibration()
        }
        
        return true
      }

      return false

    } catch (err) {
      console.error('❌ Failed to add calibration point:', err)
      return false
    }
  }

  // Finish calibration and compute transformation
  const finishCalibration = async (): Promise<boolean> => {
    try {
      console.log('🏁 Finishing calibration...')
      
      const result = await invoke('finish_calibration')
      
      if (result) {
        isCalibrated.value = true
        calibrationSession.value.is_active = false
        calibrationSession.value.quality_score = calculateCalibrationQuality()
        stats.value.calibration_quality = calibrationSession.value.quality_score
        
        console.log('✅ Calibration completed successfully')
        return true
      }

      return false

    } catch (err) {
      error.value = `Calibration finish error: ${(err as Error).message}`
      console.error('❌ Failed to finish calibration:', err)
      return false
    }
  }

  // Calculate calibration quality score
  const calculateCalibrationQuality = (): number => {
    const points = calibrationSession.value.collected_points
    if (points.length < 4) return 0

    // Simple quality metric based on point distribution and confidence
    const avgConfidence = points.reduce((sum, p) => sum + p.confidence, 0) / points.length
    const distributionScore = Math.min(points.length / 9, 1) // Normalized to 9 points
    
    return (avgConfidence * 0.7 + distributionScore * 0.3) * 100
  }

  // Update configuration
  const updateConfig = (newConfig: Partial<GazeTrackingConfig>): void => {
    config.value = { ...config.value, ...newConfig }
    console.log('⚙️ Configuration updated:', config.value)
  }

  // Get current monitor under gaze
  const getMonitorAtGaze = (): MonitorInfo | null => {
    if (!currentGaze.value || !monitorMesh.value) return null
    
    return monitorMesh.value.monitors.find(monitor =>
      currentGaze.value!.x >= monitor.x && 
      currentGaze.value!.x < monitor.x + monitor.width &&
      currentGaze.value!.y >= monitor.y && 
      currentGaze.value!.y < monitor.y + monitor.height
    ) || null
  }

  // Reset calibration
  const resetCalibration = (): void => {
    isCalibrated.value = false
    calibrationSession.value = {
      is_active: false,
      current_point: 0,
      total_points: 9,
      collected_points: [],
      target_positions: [],
      quality_score: 0
    }
    stats.value.calibration_quality = 0
  }

  // Cleanup on unmount
  onUnmounted(async () => {
    console.log('🧹 Cleaning up advanced gaze tracking...')
    await stopTracking()
  })

  // Return the composable interface
  return {
    // Core state
    isActive: readonly(isActive),
    isInitialized: readonly(isInitialized),
    isCalibrated: readonly(isCalibrated),
    currentGaze: readonly(currentGaze),
    error: readonly(error),
    isLoading: readonly(isLoading),

    // Spatial awareness
    monitorMesh: readonly(monitorMesh),
    currentMonitor: readonly(currentMonitor),
    gazeScreenPosition: readonly(gazeScreenPosition),

    // Configuration and stats
    config: readonly(config),
    stats: readonly(stats),
    calibrationSession: readonly(calibrationSession),

    // Computed properties
    isHighConfidence,
    normalizedGaze,
    trackingQuality,
    calibrationProgress,

    // Core methods
    initialize,
    startTracking,
    stopTracking,
    updateConfig,

    // Calibration methods
    startAdvancedCalibration,
    addCalibrationPoint,
    finishCalibration,
    resetCalibration,

    // Utility methods
    getMonitorAtGaze,
    loadMonitorMesh
  }
} 