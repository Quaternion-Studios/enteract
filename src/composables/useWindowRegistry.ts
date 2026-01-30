import { onUnmounted } from 'vue'
import type { 
  WindowRegistryConfig, 
  RegisteredWindow, 
  WindowId, 
  WindowRegistryState,
  WindowRegistryAPI 
} from '@/types/windowTypes'

// Global registry state - shared across all composable instances
const globalState: WindowRegistryState = {
  windows: new Map(),
  activeWindows: new Set(),
  clickHandlers: new Set()
}

let globalClickListener: ((event: MouseEvent) => void) | null = null

// Initialize global click listener once
const initializeGlobalClickListener = () => {
  if (!globalClickListener) {
    globalClickListener = (event: MouseEvent) => {
      const target = event.target as HTMLElement
      if (!target) return

      // Get all windows that should close on outside click
      const windowsToClose = Array.from(globalState.windows.values())
        .filter(window => window.config.closeOnClickOutside && window.isActive)
        .sort((a, b) => (b.config.priority || 0) - (a.config.priority || 0)) // Higher priority first

      for (const window of windowsToClose) {
        if (!window.element.contains(target)) {
          // Click is outside this window
          if (window.config.closeHandler) {
            window.config.closeHandler()
          }
          // Only close the highest priority window
          break
        }
      }
    }

    document.addEventListener('click', globalClickListener, true)
  }
}

// Cleanup global listener
const cleanupGlobalClickListener = () => {
  if (globalClickListener && globalState.windows.size === 0) {
    document.removeEventListener('click', globalClickListener, true)
    globalClickListener = null
  }
}

/**
 * Global window registry composable
 * Provides centralized window management with direct DOM element references
 */
export function useWindowRegistry(): WindowRegistryAPI {
  const register = (id: WindowId, element: HTMLElement, config: WindowRegistryConfig = {}) => {
    if (!element) {
      console.warn(`[WindowRegistry] Cannot register window '${id}' - element is null/undefined`)
      return
    }

    const defaultConfig: WindowRegistryConfig = {
      closeOnClickOutside: true,
      isModal: false,
      priority: 100,
      zIndex: undefined,
      closeHandler: undefined,
      focusHandler: undefined
    }

    const finalConfig = { ...defaultConfig, ...config }

    const registeredWindow: RegisteredWindow = {
      id,
      element,
      config: finalConfig,
      isActive: true, // Newly registered windows are considered active
      registeredAt: Date.now()
    }

    globalState.windows.set(id, registeredWindow)
    globalState.activeWindows.add(id)

    // Apply z-index if specified
    if (finalConfig.zIndex !== undefined) {
      element.style.zIndex = finalConfig.zIndex.toString()
    }

    // Initialize global click listener if needed
    initializeGlobalClickListener()

    console.log(`[WindowRegistry] Registered window '${id}'`, {
      element: element.tagName,
      config: finalConfig,
      totalWindows: globalState.windows.size
    })
  }

  const unregister = (id: WindowId) => {
    const window = globalState.windows.get(id)
    if (window) {
      globalState.windows.delete(id)
      globalState.activeWindows.delete(id)
      
      console.log(`[WindowRegistry] Unregistered window '${id}'`, {
        remainingWindows: globalState.windows.size
      })

      // Cleanup global listener if no windows remain
      cleanupGlobalClickListener()
    }
  }

  const getWindow = (id: WindowId): RegisteredWindow | undefined => {
    return globalState.windows.get(id)
  }

  const getAllWindows = (): RegisteredWindow[] => {
    return Array.from(globalState.windows.values())
  }

  const getActiveWindows = (): RegisteredWindow[] => {
    return Array.from(globalState.windows.values()).filter(w => w.isActive)
  }

  const isRegistered = (id: WindowId): boolean => {
    return globalState.windows.has(id)
  }

  const isActive = (id: WindowId): boolean => {
    return globalState.activeWindows.has(id)
  }

  const setActive = (id: WindowId) => {
    const window = globalState.windows.get(id)
    if (window) {
      window.isActive = true
      globalState.activeWindows.add(id)
      
      if (window.config.focusHandler) {
        window.config.focusHandler()
      }
    }
  }

  const setInactive = (id: WindowId) => {
    const window = globalState.windows.get(id)
    if (window) {
      window.isActive = false
      globalState.activeWindows.delete(id)
    }
  }

  const bringToFront = (id: WindowId) => {
    const window = globalState.windows.get(id)
    if (window && window.config.zIndex !== undefined) {
      // Find the highest z-index among all windows
      const maxZIndex = Math.max(
        ...Array.from(globalState.windows.values())
          .map(w => parseInt(w.element.style.zIndex || '0'))
          .filter(z => !isNaN(z))
      )
      
      window.element.style.zIndex = (maxZIndex + 1).toString()
    }
  }

  const isClickOutside = (target: HTMLElement, windowId?: WindowId): boolean => {
    if (windowId) {
      const window = globalState.windows.get(windowId)
      return window ? !window.element.contains(target) : true
    }
    return false
  }

  const isClickOutsideAll = (target: HTMLElement): boolean => {
    return Array.from(globalState.windows.values()).every(window => 
      !window.element.contains(target)
    )
  }

  const cleanup = () => {
    globalState.windows.clear()
    globalState.activeWindows.clear()
    cleanupGlobalClickListener()
  }

  return {
    register,
    unregister,
    getWindow,
    getAllWindows,
    getActiveWindows,
    isRegistered,
    isActive,
    setActive,
    setInactive,
    bringToFront,
    isClickOutside,
    isClickOutsideAll,
    cleanup
  }
}

/**
 * Convenience composable for individual window components
 * Automatically handles registration/unregistration lifecycle
 */
export function useWindowRegistration(
  windowId: WindowId, 
  config?: WindowRegistryConfig
) {
  const registry = useWindowRegistry()
  let currentElement: HTMLElement | null = null

  const registerSelf = (element: HTMLElement) => {
    if (currentElement && currentElement !== element) {
      // Unregister old element first
      registry.unregister(windowId)
    }
    
    currentElement = element
    registry.register(windowId, element, config)
  }

  const unregisterSelf = () => {
    if (currentElement) {
      registry.unregister(windowId)
      currentElement = null
    }
  }

  const updateConfig = (newConfig: WindowRegistryConfig) => {
    if (currentElement) {
      registry.unregister(windowId)
      registry.register(windowId, currentElement, { ...config, ...newConfig })
    }
  }

  // Auto-cleanup on unmount
  onUnmounted(() => {
    unregisterSelf()
  })

  return {
    registerSelf,
    unregisterSelf,
    updateConfig,
    ...registry
  }
}
