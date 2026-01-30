import { ref, computed, onUnmounted } from 'vue'
import { Window } from '@tauri-apps/api/window'
import { LogicalSize, LogicalPosition } from '@tauri-apps/api/dpi'
import { useAppStore } from '../stores/app'
import { invoke } from '@tauri-apps/api/core'
import type { MonitorInfo } from '../types/monitor'
// Type definitions for window management
interface Point2D {
  x: number
  y: number
}

interface Rectangle {
  x: number
  y: number
  width: number
  height: number
}

interface WindowState {
  position: Point2D
  size: { width: number; height: number }
  screenSize: { width: number; height: number }
  monitors: MonitorInfo[]
  currentMonitor: MonitorInfo | null
  homeMonitor: MonitorInfo | null
  isMoving: boolean
  lastMoveTime: number
}

interface MovementConfig {
  enabled: boolean
  sensitivity: number        // 0.1 to 2.0, how responsive to gaze
  smoothing: number         // 0.1 to 0.9, higher = smoother but slower
  deadZone: number          // 0.05 to 0.3, center area that doesn't trigger movement
  maxSpeed: number          // pixels per frame
  minDistance: number       // minimum pixels to move
  edgeBuffer: number        // pixels to keep from screen edge
}

export function useWindowManager() {
  const store = useAppStore()
  const currentWindow = Window.getCurrent()
  
  const isDragging = ref(false)
  const isResizing = ref(false)
  
  // Window dimensions for collapsed state
  const normalSize = { width: 800, height: 600 }
  const collapsedSize = { width: 300, height: 80 }

  // State
  const state = ref<WindowState>({
    position: { x: 0, y: 0 },
    size: { width: 400, height: 300 },
    screenSize: { width: 1920, height: 1080 },
    monitors: [],
    currentMonitor: null,
    homeMonitor: null,
    isMoving: false,
    lastMoveTime: 0
  })

  const config = ref<MovementConfig>({
    enabled: false,
    sensitivity: 1.0,
    smoothing: 0.7,
    deadZone: 0.15,
    maxSpeed: 20,
    minDistance: 3,
    edgeBuffer: 50
  })

  // Movement smoothing
  const movementHistory = ref<Point2D[]>([])
  const smoothingQueue = ref<Point2D[]>([])

  const initializeWindow = async () => {
    try {
      // Set initial window properties
      await currentWindow.setDecorations(false)
      await currentWindow.setAlwaysOnTop(true)
      await currentWindow.setSize(new LogicalSize(normalSize.width, normalSize.height))
      
      // Center window initially
      const screenSize = await currentWindow.outerSize()
      const position = new LogicalPosition(
        Math.floor((screenSize.width - normalSize.width) / 2),
        Math.floor((screenSize.height - normalSize.height) / 2)
      )
      await currentWindow.setPosition(position)
      store.updateWindowPosition(position.x, position.y)
      
    } catch (error) {
      console.error('Failed to initialize window:', error)
    }
  }

  const toggleCollapse = async () => {
    try {
      store.toggleWindowCollapse()
      
      if (store.windowCollapsed) {
        await currentWindow.setSize(new LogicalSize(collapsedSize.width, collapsedSize.height))
      } else {
        await currentWindow.setSize(new LogicalSize(normalSize.width, normalSize.height))
      }
    } catch (error) {
      console.error('Failed to toggle window collapse:', error)
    }
  }

  const startDrag = async () => {
    try {
      isDragging.value = true
      await currentWindow.startDragging()
    } catch (error) {
      console.error('Failed to start dragging:', error)
    } finally {
      isDragging.value = false
    }
  }

  const minimizeWindow = async () => {
    try {
      console.log('Attempting to minimize window...')
      
      // Try the minimize method
      await currentWindow.minimize()
      console.log('Window minimized successfully')
      
    } catch (error) {
      console.error('Failed to minimize window:', error)
      console.error('Error details:', error)
      
      // Fallback: try to hide the window
      try {
        console.log('Trying fallback: hiding window...')
        await currentWindow.hide()
        console.log('Window hidden successfully as fallback')
      } catch (hideError) {
        console.error('Failed to hide window as fallback:', hideError)
        
        // Last resort: try setting size to very small
        try {
          console.log('Last resort: minimizing by size...')
          await currentWindow.setSize(new LogicalSize(1, 1))
          await currentWindow.setPosition(new LogicalPosition(-1000, -1000))
        } catch (sizeError) {
          console.error('All minimize attempts failed:', sizeError)
        }
      }
    }
  }

  const closeWindow = async () => {
    try {
      console.log('Attempting to close window...')
      
      // Try the close method
      await currentWindow.close()
      console.log('Window closed successfully')
      
    } catch (error) {
      console.error('Failed to close window:', error)
      console.error('Error details:', error)
      
      // Try destroy as fallback
      try {
        console.log('Trying fallback: destroying window...')
        await currentWindow.destroy()
        console.log('Window destroyed successfully as fallback')
      } catch (destroyError) {
        console.error('Failed to destroy window as fallback:', destroyError)
        
        // Final fallback: try to exit the app
        try {
          console.log('Final fallback: attempting app exit...')
          // In a real Tauri app, you might use process.exit() or tauri.exit()
          window.close()
        } catch (exitError) {
          console.error('All close attempts failed:', exitError)
        }
      }
    }
  }

  // Monitor management functions
  const loadMonitorLayout = async (): Promise<void> => {
    try {
      const monitors = await invoke<MonitorInfo[]>('get_monitor_layout')
      state.value.monitors = monitors
      
      // Find the smallest monitor (home monitor)
      const homeMonitor = monitors.reduce((smallest, monitor) => {
        const area = monitor.width * monitor.height
        const smallestArea = smallest.width * smallest.height
        return area < smallestArea ? monitor : smallest
      })
      
      state.value.homeMonitor = homeMonitor
      console.log('🏠 Home monitor set:', homeMonitor)
      console.log('📊 Detected monitors:', monitors)
    } catch (error) {
      console.error('Failed to load monitor layout:', error)
      // Fallback to single monitor
      const [virtualWidth, virtualHeight] = await invoke<[number, number]>('get_virtual_desktop_size')
      state.value.monitors = [{
        x: 0,
        y: 0,
        width: virtualWidth,
        height: virtualHeight,
        is_primary: true,
        name: 'Primary'
      }]
      state.value.homeMonitor = state.value.monitors[0]
    }
  }

  const getMonitorAt = (x: number, y: number): MonitorInfo | null => {
    return state.value.monitors.find(monitor => 
      x >= monitor.x && x < monitor.x + monitor.width &&
      y >= monitor.y && y < monitor.y + monitor.height
    ) || null
  }

  const moveToHomePosition = async (): Promise<void> => {
    if (!state.value.homeMonitor) return
    
    const monitor = state.value.homeMonitor
    // Position in upper third of home monitor
    const targetX = monitor.x + (monitor.width - state.value.size.width) / 2
    const targetY = monitor.y + monitor.height / 4  // Upper quarter
    
    await moveWindow({ x: targetX, y: targetY })
    console.log('🏠 Moved to home position:', { x: targetX, y: targetY })
  }

  // Initialize window state
  const initializeWindowState = async (): Promise<void> => {
    try {
      // Load monitor layout first
      await loadMonitorLayout()
      
      // Get current window position and size, and virtual desktop size for multi-monitor support
      const [position, size, screenSize] = await Promise.all([
        invoke<[number, number]>('get_window_position'),
        invoke<[number, number]>('get_window_size'),
        invoke<[number, number]>('get_virtual_desktop_size')  // Use virtual desktop for multi-monitor
      ])

      state.value.position = { x: position[0], y: position[1] }
      state.value.size = { width: size[0], height: size[1] }
      state.value.screenSize = { width: screenSize[0], height: screenSize[1] }

      console.log('Window initialized (multi-monitor aware):', state.value)
    } catch (error) {
      console.error('Failed to initialize window state:', error)
    }
  }

  // Convert gaze coordinates (-1 to 1) to screen coordinates
  const gazeToScreenCoordinates = (gaze: Point2D): Point2D => {
    const { screenSize } = state.value
    
    // Convert normalized gaze (-1 to 1) to screen coordinates (0 to screen size)
    const screenX = ((gaze.x + 1) / 2) * screenSize.width
    const screenY = ((gaze.y + 1) / 2) * screenSize.height
    
    return { x: screenX, y: screenY }
  }

  // Calculate target window position based on gaze
  const calculateTargetPosition = (gazeScreenPos: Point2D): Point2D => {
    const { size, screenSize } = state.value
    const { edgeBuffer } = config.value

    // Center the window on the gaze point
    let targetX = gazeScreenPos.x - (size.width / 2)
    let targetY = gazeScreenPos.y - (size.height / 2)

    // Apply screen boundaries with edge buffer
    targetX = Math.max(edgeBuffer, Math.min(targetX, screenSize.width - size.width - edgeBuffer))
    targetY = Math.max(edgeBuffer, Math.min(targetY, screenSize.height - size.height - edgeBuffer))

    return { x: Math.round(targetX), y: Math.round(targetY) }
  }

  // Apply dead zone to prevent micro-movements
  const applyDeadZone = (gaze: Point2D): Point2D | null => {
    const distance = Math.sqrt(gaze.x * gaze.x + gaze.y * gaze.y)
    
    if (distance < config.value.deadZone) {
      return null // Within dead zone, no movement
    }

    return gaze
  }

  // Apply smoothing to reduce jitter
  const applySmoothingFilter = (newPosition: Point2D): Point2D => {
    const { smoothing } = config.value
    const maxHistory = 5

    // Add to smoothing queue
    smoothingQueue.value.push(newPosition)
    if (smoothingQueue.value.length > maxHistory) {
      smoothingQueue.value.shift()
    }

    // Calculate weighted average with recent positions having more weight
    let weightedX = 0
    let weightedY = 0
    let totalWeight = 0

    smoothingQueue.value.forEach((pos: Point2D, index: number) => {
      const weight = Math.pow(smoothing, smoothingQueue.value.length - index - 1)
      weightedX += pos.x * weight
      weightedY += pos.y * weight
      totalWeight += weight
    })

    return {
      x: Math.round(weightedX / totalWeight),
      y: Math.round(weightedY / totalWeight)
    }
  }

  // Check if movement meets minimum distance threshold
  const shouldMove = (newPosition: Point2D): boolean => {
    const { position } = state.value
    const { minDistance } = config.value

    const distance = Math.sqrt(
      Math.pow(newPosition.x - position.x, 2) + 
      Math.pow(newPosition.y - position.y, 2)
    )

    return distance >= minDistance
  }

  // Execute window movement with speed limiting
  const moveWindow = async (targetPos: Point2D): Promise<void> => {
    const { position } = state.value
    const { maxSpeed } = config.value

    // Calculate movement vector
    const deltaX = targetPos.x - position.x
    const deltaY = targetPos.y - position.y
    const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY)

    if (distance === 0) return

    // Limit movement speed
    let moveX = deltaX
    let moveY = deltaY

    if (distance > maxSpeed) {
      const ratio = maxSpeed / distance
      moveX = deltaX * ratio
      moveY = deltaY * ratio
    }

    const newPosition = {
      x: Math.round(position.x + moveX),
      y: Math.round(position.y + moveY)
    }

    try {
      state.value.isMoving = true
      await invoke('move_window_to_position', { x: newPosition.x, y: newPosition.y })
      
      // Update state
      state.value.position = newPosition
      state.value.lastMoveTime = Date.now()
      
      // Add to movement history
      movementHistory.value.push(newPosition)
      if (movementHistory.value.length > 10) {
        movementHistory.value.shift()
      }

    } catch (error) {
      console.error('Failed to move window:', error)
    } finally {
      state.value.isMoving = false
    }
  }

  // Main function to process gaze input and move window
  const processGazeInput = async (gaze: Point2D): Promise<void> => {
    if (!config.value.enabled) return

    try {
      // Apply dead zone filter
      const filteredGaze = applyDeadZone(gaze)
      if (!filteredGaze) return

      // Convert to screen coordinates
      const screenPos = gazeToScreenCoordinates(filteredGaze)
      
      // Detect which monitor the gaze is on (for monitor switching)
      const gazeMonitor = getMonitorAt(screenPos.x, screenPos.y)
      if (gazeMonitor && gazeMonitor !== state.value.currentMonitor) {
        state.value.currentMonitor = gazeMonitor
        console.log(`🖥️ Switched to monitor: ${gazeMonitor.name}`)
      }
      
      // Calculate target window position
      const rawTargetPos = calculateTargetPosition(screenPos)
      
      // Apply smoothing
      const smoothedPos = applySmoothingFilter(rawTargetPos)
      
      // Check if movement is significant enough
      if (!shouldMove(smoothedPos)) return

      // Execute movement (allows cross-monitor movement)
      await moveWindow(smoothedPos)

    } catch (error) {
      console.error('Error processing gaze input:', error)
    }
  }

  // Enable/disable gaze-controlled window movement
  const enableGazeControl = async (): Promise<void> => {
    await initializeWindowState()
    config.value.enabled = true
    console.log('Gaze-controlled window movement enabled')
  }

  const disableGazeControl = (): void => {
    config.value.enabled = false
    smoothingQueue.value = []
    movementHistory.value = []
    console.log('Gaze-controlled window movement disabled')
  }

  // Update configuration
  const updateConfig = (newConfig: Partial<MovementConfig>): void => {
    config.value = { ...config.value, ...newConfig }
  }

  // Get current window bounds as rectangle
  const getWindowBounds = computed((): Rectangle => ({
    x: state.value.position.x,
    y: state.value.position.y,
    width: state.value.size.width,
    height: state.value.size.height
  }))

  // Calculate movement statistics
  const movementStats = computed(() => {
    const recentMoves = movementHistory.value.slice(-5)
    if (recentMoves.length < 2) return null

    let totalDistance = 0
    for (let i = 1; i < recentMoves.length; i++) {
      const prev = recentMoves[i - 1]
      const curr = recentMoves[i]
      totalDistance += Math.sqrt(
        Math.pow(curr.x - prev.x, 2) + Math.pow(curr.y - prev.y, 2)
      )
    }

    return {
      averageDistance: totalDistance / (recentMoves.length - 1),
      totalMoves: movementHistory.value.length,
      isActive: Date.now() - state.value.lastMoveTime < 1000
    }
  })

  // Cleanup
  onUnmounted(() => {
    disableGazeControl()
  })

  // Initialize on mount
  initializeWindow()
  
  // Move to home position after initialization
  setTimeout(() => {
    moveToHomePosition()
  }, 1000)

  return {
    isDragging,
    isResizing,
    normalSize,
    collapsedSize,
    initializeWindow,
    toggleCollapse,
    startDrag,
    minimizeWindow,
    closeWindow,
    state: computed(() => state.value),
    config: computed(() => config.value),
    windowBounds: getWindowBounds,
    movementStats,
    processGazeInput,
    enableGazeControl,
    disableGazeControl,
    updateConfig,
    initializeWindowState,
    gazeToScreenCoordinates,
    calculateTargetPosition,
    loadMonitorLayout,
    getMonitorAt,
    moveToHomePosition,
    isEnabled: computed(() => config.value.enabled),
    isMoving: computed(() => state.value.isMoving)
  }
} 