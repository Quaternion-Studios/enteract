import { ref, computed, onUnmounted, readonly } from 'vue'
import { invoke } from '@tauri-apps/api/core'
// import { KalmanFilter2D, type KalmanConfig } from '../lib/filters/KalmanFilter' // TODO: Add KalmanFilter implementation

// Types for ML eye tracking
export interface MLGazeData {
  x: number
  y: number
  confidence: number
  timestamp: number
  calibrated: boolean
}

export interface MLEyeTrackingConfig {
  camera_id: number
  screen_width: number
  screen_height: number
  model_path?: string
  smoothing_window: number
  confidence_threshold: number
  kalman_process_noise: number
  kalman_measurement_noise: number
  adaptive_smoothing: boolean
}

export interface MLTrackingStats {
  status: string
  model_type: string
  features: string[]
  performance: {
    expected_fps: string
    latency_ms: string
    accuracy: string
  }
}

export interface CalibrationPoint {
  screenX: number
  screenY: number
  gazeX: number
  gazeY: number
  confidence: number
  timestamp: number
}

export interface SmoothingStats {
  rawGaze: { x: number, y: number }
  smoothedGaze: { x: number, y: number }
  confidence: number
  smoothingStrength: number
  outlierDetected: boolean
  kalmanVelocity: { vx: number, vy: number }
}

export function useMLEyeTracking() {
  // State
  const isActive = ref(false)
  const currentGaze = ref<MLGazeData | null>(null)
  const smoothedGaze = ref<MLGazeData | null>(null)
  const isCalibrated = ref(false)
  const trackingStats = ref<MLTrackingStats | null>(null)
  const error = ref<string | null>(null)
  const isLoading = ref(false)

  // Enhanced configuration (will be updated with virtual desktop dimensions)
  const config = ref<MLEyeTrackingConfig>({
    camera_id: 0,
    screen_width: 0,  // Will be set by resetConfig()
    screen_height: 0, // Will be set by resetConfig()
    smoothing_window: 8,
    confidence_threshold: 0.7,
    kalman_process_noise: 0.1,
    kalman_measurement_noise: 1.0,
    adaptive_smoothing: true
  })

  // Initialize config with virtual desktop dimensions
  resetConfig()

  // Performance metrics
  const fps = ref(0)
  const accuracy = ref(0)
  const lastUpdateTime = ref(0)
  const smoothingStats = ref<SmoothingStats | null>(null)

  // Tracking loop management
  let trackingInterval: number | null = null
  let fpsInterval: number | null = null
  let frameCount = 0

  // Smoothing components
  const kalmanFilter = ref<any | null>(null) // TODO: Replace with KalmanFilter2D type when available
  const gazeHistory = ref<Array<MLGazeData>>([])
  const maxHistorySize = 10

  // Calibration data
  const calibrationPoints = ref<CalibrationPoint[]>([])
  const calibrationOffsets = ref({ x: 0, y: 0 })
  const calibrationScale = ref({ x: 1, y: 1 })

  // Computed properties
  const gazeScreenPosition = computed(() => {
    if (!smoothedGaze.value) return null
    return {
      x: Math.max(0, Math.min(smoothedGaze.value.x, config.value.screen_width)),
      y: Math.max(0, Math.min(smoothedGaze.value.y, config.value.screen_height))
    }
  })

  const normalizedGaze = computed(() => {
    if (!gazeScreenPosition.value) return null
    return {
      x: gazeScreenPosition.value.x / config.value.screen_width,
      y: gazeScreenPosition.value.y / config.value.screen_height
    }
  })

  const confidence = computed(() => {
    return smoothedGaze.value?.confidence || 0
  })

  const isHighConfidence = computed(() => {
    return confidence.value > config.value.confidence_threshold
  })

  const smoothingEnabled = computed(() => {
    return kalmanFilter.value !== null
  })

  // Initialize smoothing components
  function initializeSmoothingComponents() {
    // Kalman filter disabled to match original gaze-tracker.py motion characteristics
    // Only using simple moving average smoothing now
    kalmanFilter.value = null
    
    // Clear history
    gazeHistory.value = []
    calibrationPoints.value = []
    
    console.log('🎯 Simple smoothing initialized (no Kalman filter)')
  }

  // Core smoothing pipeline
  function applySmoothingPipeline(rawGaze: MLGazeData): MLGazeData {
    const startTime = performance.now()
    
    // Step 1: Confidence-based filtering
    if (rawGaze.confidence < config.value.confidence_threshold) {
      console.log(`🚫 Low confidence gaze rejected: ${rawGaze.confidence.toFixed(2)}`)
      return rawGaze // Return raw data but don't update smoothed value
    }

    // Step 2: Outlier detection
    const isOutlier = detectOutlier(rawGaze)
    if (isOutlier) {
      console.log('🚫 Outlier gaze detected and rejected')
      return rawGaze
    }

    // Step 3: Apply calibration corrections
    const calibratedGaze = applyCalibrationCorrections(rawGaze)

    // Step 4: Skip Kalman filtering (use original simple smoothing)
    // Kalman filter disabled to match original gaze-tracker.py motion characteristics
    let kalmanSmoothed = calibratedGaze

    // Step 5: Moving average smoothing (adaptive)
    const movingAverageSmoothed = applyMovingAverageSmoothing(kalmanSmoothed)

    // Step 6: Update statistics
    updateSmoothingStats(rawGaze, movingAverageSmoothed)

    const processingTime = performance.now() - startTime
    console.log(`⚡ Smoothing pipeline: ${processingTime.toFixed(2)}ms`)

    return movingAverageSmoothed
  }

  // Outlier detection using distance threshold
  function detectOutlier(gaze: MLGazeData): boolean {
    if (gazeHistory.value.length < 3) return false

    const recentGazes = gazeHistory.value.slice(-3)
    const avgX = recentGazes.reduce((sum, g) => sum + g.x, 0) / recentGazes.length
    const avgY = recentGazes.reduce((sum, g) => sum + g.y, 0) / recentGazes.length
    
    const distance = Math.sqrt(
      Math.pow(gaze.x - avgX, 2) + Math.pow(gaze.y - avgY, 2)
    )
    
    // Dynamic threshold based on screen size
    const threshold = Math.min(config.value.screen_width, config.value.screen_height) * 0.3
    
    return distance > threshold
  }

  // Apply calibration corrections
  function applyCalibrationCorrections(gaze: MLGazeData): MLGazeData {
    if (!isCalibrated.value) return gaze

    return {
      ...gaze,
      x: (gaze.x + calibrationOffsets.value.x) * calibrationScale.value.x,
      y: (gaze.y + calibrationOffsets.value.y) * calibrationScale.value.y,
      calibrated: true
    }
  }

  // Simple moving average smoothing (matching original gaze-tracker.py)
  function applyMovingAverageSmoothing(gaze: MLGazeData): MLGazeData {
    // Add to history (limit to 5 samples like original)
    gazeHistory.value.push(gaze)
    if (gazeHistory.value.length > 5) {
      gazeHistory.value.shift()
    }

    // Apply simple moving average if we have at least 2 samples (like original)
    if (gazeHistory.value.length >= 2) {
      const avgX = gazeHistory.value.reduce((sum, g) => sum + g.x, 0) / gazeHistory.value.length
      const avgY = gazeHistory.value.reduce((sum, g) => sum + g.y, 0) / gazeHistory.value.length

      return {
        ...gaze,
        x: avgX,
        y: avgY
      }
    }

    return gaze
  }

  // Update smoothing statistics
  function updateSmoothingStats(rawGaze: MLGazeData, smoothedGaze: MLGazeData) {
    const velocity = kalmanFilter.value?.getVelocity() || { vx: 0, vy: 0 }
    
    smoothingStats.value = {
      rawGaze: { x: rawGaze.x, y: rawGaze.y },
      smoothedGaze: { x: smoothedGaze.x, y: smoothedGaze.y },
      confidence: smoothedGaze.confidence,
      smoothingStrength: 0.3, // Will be dynamic based on movement
      outlierDetected: detectOutlier(rawGaze),
      kalmanVelocity: velocity
    }
  }

  // Enhanced calibration system with 9-point grid
  async function performNinePointCalibration(): Promise<boolean> {
    if (!isActive.value) {
      error.value = 'Cannot calibrate: tracking not active'
      return false
    }

    try {
      isLoading.value = true
      calibrationPoints.value = []
      
      console.log('🎯 Starting 9-point calibration...')
      
      // Define 9 calibration points (3x3 grid)
      const points = [
        { x: 0.1, y: 0.1 }, { x: 0.5, y: 0.1 }, { x: 0.9, y: 0.1 },
        { x: 0.1, y: 0.5 }, { x: 0.5, y: 0.5 }, { x: 0.9, y: 0.5 },
        { x: 0.1, y: 0.9 }, { x: 0.5, y: 0.9 }, { x: 0.9, y: 0.9 }
      ]

      // Emit calibration start event for UI
      const event = new CustomEvent('calibration-started', { 
        detail: { points: points.length } 
      })
      window.dispatchEvent(event)

      for (let i = 0; i < points.length; i++) {
        const screenX = points[i].x * config.value.screen_width
        const screenY = points[i].y * config.value.screen_height
        
        // Emit calibration point event
        const pointEvent = new CustomEvent('calibration-point', {
          detail: { index: i, screenX, screenY, total: points.length }
        })
        window.dispatchEvent(pointEvent)
        
        // Wait for user to look at point
        await new Promise(resolve => setTimeout(resolve, 2000))
        
        // Collect gaze data for this point
        await collectCalibrationData(screenX, screenY)
      }
      
      // Calculate calibration corrections
      calculateCalibrationCorrections()
      
      isCalibrated.value = true
      
      const successEvent = new CustomEvent('calibration-complete', {
        detail: { points: calibrationPoints.value.length }
      })
      window.dispatchEvent(successEvent)
      
      console.log('✅ 9-point calibration completed successfully')
      return true
      
    } catch (err) {
      console.error('Failed to perform calibration:', err)
      error.value = err as string
      return false
    } finally {
      isLoading.value = false
    }
  }

  // Collect calibration data for a specific point
  async function collectCalibrationData(screenX: number, screenY: number) {
    const samples: MLGazeData[] = []
    const sampleCount = 30 // Collect 30 samples over 1 second
    
    for (let i = 0; i < sampleCount; i++) {
      if (currentGaze.value && currentGaze.value.confidence > 0.5) {
        samples.push({ ...currentGaze.value })
      }
      await new Promise(resolve => setTimeout(resolve, 33)) // ~30fps
    }
    
    if (samples.length > 10) {
      // Calculate average gaze position
      const avgX = samples.reduce((sum, s) => sum + s.x, 0) / samples.length
      const avgY = samples.reduce((sum, s) => sum + s.y, 0) / samples.length
      const avgConfidence = samples.reduce((sum, s) => sum + s.confidence, 0) / samples.length
      
      calibrationPoints.value.push({
        screenX,
        screenY,
        gazeX: avgX,
        gazeY: avgY,
        confidence: avgConfidence,
        timestamp: Date.now()
      })
      
      console.log(`📍 Calibration point collected: screen(${screenX.toFixed(0)}, ${screenY.toFixed(0)}) -> gaze(${avgX.toFixed(0)}, ${avgY.toFixed(0)})`)
    }
  }

  // Calculate calibration corrections using least squares
  function calculateCalibrationCorrections() {
    if (calibrationPoints.value.length < 5) {
      console.warn('Insufficient calibration points for correction calculation')
      return
    }
    
    // Simple linear correction calculation
    let sumScreenX = 0, sumScreenY = 0, sumGazeX = 0, sumGazeY = 0
    let sumScreenXGazeX = 0, sumScreenYGazeY = 0
    let sumScreenX2 = 0, sumScreenY2 = 0
    
    calibrationPoints.value.forEach(point => {
      sumScreenX += point.screenX
      sumScreenY += point.screenY
      sumGazeX += point.gazeX
      sumGazeY += point.gazeY
      sumScreenXGazeX += point.screenX * point.gazeX
      sumScreenYGazeY += point.screenY * point.gazeY
      sumScreenX2 += point.screenX * point.screenX
      sumScreenY2 += point.screenY * point.screenY
    })
    
    const n = calibrationPoints.value.length
    
    // Calculate scale factors
    const scaleX = (n * sumScreenXGazeX - sumScreenX * sumGazeX) / (n * sumScreenX2 - sumScreenX * sumScreenX)
    const scaleY = (n * sumScreenYGazeY - sumScreenY * sumGazeY) / (n * sumScreenY2 - sumScreenY * sumScreenY)
    
    // Calculate offsets
    const offsetX = (sumGazeX - scaleX * sumScreenX) / n
    const offsetY = (sumGazeY - scaleY * sumScreenY) / n
    
    calibrationScale.value = { x: 1/scaleX, y: 1/scaleY }
    calibrationOffsets.value = { x: -offsetX, y: -offsetY }
    
    console.log('🎯 Calibration corrections calculated:', {
      scale: calibrationScale.value,
      offset: calibrationOffsets.value
    })
  }

  // Core functions with enhanced smoothing
  async function startTracking(userConfig?: Partial<MLEyeTrackingConfig>) {
    if (isActive.value) {
      console.warn('ML Eye tracking already active')
      return
    }

    try {
      isLoading.value = true
      error.value = null

      // Ensure config has correct virtual desktop dimensions
      if (!config.value.screen_width || !config.value.screen_height) {
        await resetConfig()
      }

      // Update configuration
      if (userConfig) {
        config.value = { ...config.value, ...userConfig }
      }

      // Initialize smoothing components
      initializeSmoothingComponents()

      // Start the ML eye tracking process
      const result = await invoke<string>('start_ml_eye_tracking', { 
        config: config.value 
      })
      
      console.log('✅ ML Eye tracking started:', result)
      console.log('🔧 Configuration:', config.value)
      
      isActive.value = true
      
      // Start data polling loop
      startDataPolling()
      
      // Start FPS monitoring
      startFPSMonitoring()
      
      // Get initial stats
      await updateStats()
      
      console.log('🚀 ML Eye tracking with smoothing fully initialized!')

    } catch (err) {
      console.error('Failed to start ML eye tracking:', err)
      error.value = err as string
    } finally {
      isLoading.value = false
    }
  }

  // Enhanced data polling with smoothing
  function startDataPolling() {
    if (trackingInterval) return

    console.log('🔄 Starting ML data polling with smoothing at 30 FPS...')
    
    trackingInterval = window.setInterval(async () => {
      try {
        const gazeData = await invoke<MLGazeData | null>('get_ml_gaze_data')
        
        if (gazeData) {
          currentGaze.value = gazeData
          lastUpdateTime.value = Date.now()
          frameCount++
          
          // Apply smoothing pipeline
          const smooth = applySmoothingPipeline(gazeData)
          smoothedGaze.value = smooth
          
          // Update accuracy based on confidence
          accuracy.value = smooth.confidence
          
          // Log detailed data periodically
          if (frameCount % 30 === 0) { // Every 30 frames (~1 second)
            console.log('👁️ Gaze Data:', {
              raw: `(${gazeData.x.toFixed(0)}, ${gazeData.y.toFixed(0)})`,
              smoothed: `(${smooth.x.toFixed(0)}, ${smooth.y.toFixed(0)})`,
              confidence: `${(smooth.confidence * 100).toFixed(1)}%`,
              calibrated: smooth.calibrated
            })
          }
        }
      } catch (err) {
        console.error('Failed to get ML gaze data:', err)
      }
    }, 50) // ~20 FPS (matching original gaze-tracker.py)
  }

  async function stopTracking() {
    if (!isActive.value) return

    try {
      isLoading.value = true
      
      // Stop data polling
      stopDataPolling()
      
      // Stop the ML tracking process
      const result = await invoke<string>('stop_ml_eye_tracking')
      console.log('ML Eye tracking stopped:', result)
      
      isActive.value = false
      currentGaze.value = null
      isCalibrated.value = false
      
    } catch (err) {
      console.error('Failed to stop ML eye tracking:', err)
      error.value = err as string
    } finally {
      isLoading.value = false
    }
  }

  async function calibrate() {
    if (!isActive.value) {
      error.value = 'Cannot calibrate: tracking not active'
      return false
    }

    try {
      isLoading.value = true
      
      const result = await invoke<string>('calibrate_ml_eye_tracking')
      console.log('ML Eye tracking calibration:', result)
      
      // Wait a moment for calibration to complete
      await new Promise(resolve => setTimeout(resolve, 2000))
      
      isCalibrated.value = true
      return true
      
    } catch (err) {
      console.error('Failed to calibrate ML eye tracking:', err)
      error.value = err as string
      return false
    } finally {
      isLoading.value = false
    }
  }

  function stopDataPolling() {
    if (trackingInterval) {
      clearInterval(trackingInterval)
      trackingInterval = null
    }
    
    if (fpsInterval) {
      clearInterval(fpsInterval)
      fpsInterval = null
    }
  }

  function startFPSMonitoring() {
    frameCount = 0
    
    fpsInterval = window.setInterval(() => {
      fps.value = frameCount
      frameCount = 0
    }, 1000)
  }

  async function updateStats() {
    try {
      const stats = await invoke<MLTrackingStats>('get_ml_tracking_stats')
      trackingStats.value = stats
    } catch (err) {
      console.error('Failed to get ML tracking stats:', err)
    }
  }

  // Movement detection
  const previousGaze = ref<MLGazeData | null>(null)
  const movementThreshold = 50 // pixels

  const hasSignificantMovement = computed(() => {
    if (!currentGaze.value || !previousGaze.value) return false
    
    const dx = currentGaze.value.x - previousGaze.value.x
    const dy = currentGaze.value.y - previousGaze.value.y
    const distance = Math.sqrt(dx * dx + dy * dy)
    
    return distance > movementThreshold
  })

  // Update previous gaze for movement detection
  function updatePreviousGaze() {
    if (currentGaze.value) {
      previousGaze.value = { ...currentGaze.value }
    }
  }

  // Configuration helpers
  function updateConfig(newConfig: Partial<MLEyeTrackingConfig>) {
    config.value = { ...config.value, ...newConfig }
  }

  async function resetConfig() {
    try {
      // Get virtual desktop size for multi-monitor support
      const [virtualWidth, virtualHeight] = await invoke<[number, number]>('get_virtual_desktop_size')
      
      config.value = {
        camera_id: 0,
        screen_width: virtualWidth,
        screen_height: virtualHeight,
        smoothing_window: 8,
        confidence_threshold: 0.7,
        kalman_process_noise: 0.1,
        kalman_measurement_noise: 1.0,
        adaptive_smoothing: true
      }
    } catch (error) {
      console.warn('Failed to get virtual desktop size, using window screen dimensions', error)
      config.value = {
        camera_id: 0,
        screen_width: window.screen.width,
        screen_height: window.screen.height,
        smoothing_window: 8,
        confidence_threshold: 0.7,
        kalman_process_noise: 0.1,
        kalman_measurement_noise: 1.0,
        adaptive_smoothing: true
      }
    }
  }

  // Performance optimization
  const averageConfidence = computed(() => {
    if (gazeHistory.value.length === 0) return 0
    const sum = gazeHistory.value.reduce((acc, gaze) => acc + gaze.confidence, 0)
    return sum / gazeHistory.value.length
  })

  // Stability detection
  const isStable = computed(() => {
    if (gazeHistory.value.length < 10) return false
    
    const recent = gazeHistory.value.slice(-10)
    const avgX = recent.reduce((acc, g) => acc + g.x, 0) / recent.length
    const avgY = recent.reduce((acc, g) => acc + g.y, 0) / recent.length
    
    const variance = recent.reduce((acc, g) => {
      const dx = g.x - avgX
      const dy = g.y - avgY
      return acc + (dx * dx + dy * dy)
    }, 0) / recent.length
    
    return variance < 1000 // Threshold for stability
  })

  // Cleanup
  onUnmounted(() => {
    stopTracking()
  })

  // Return the composable interface
  return {
    // State
    isActive: readonly(isActive),
    currentGaze: readonly(currentGaze),
    smoothedGaze: readonly(smoothedGaze),
    isCalibrated: readonly(isCalibrated),
    trackingStats: readonly(trackingStats),
    error: readonly(error),
    isLoading: readonly(isLoading),
    
    // Computed
    gazeScreenPosition,
    normalizedGaze,
    confidence,
    isHighConfidence,
    hasSignificantMovement,
    isStable,
    averageConfidence,
    smoothingEnabled,
    
    // Metrics
    fps: readonly(fps),
    accuracy: readonly(accuracy),
    
    // Configuration
    config: readonly(config),
    updateConfig,
    resetConfig,
    
    // Core functions
    startTracking,
    stopTracking,
    calibrate,
    updateStats,
    updatePreviousGaze,
    
    // Smoothing
    smoothingStats,
    performNinePointCalibration,
    
    // Utilities
  }
} 