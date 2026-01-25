import { useControlPanelState } from './useControlPanelState'
import { useWindowRegistry } from './useWindowRegistry'
import { useWindowResizing } from './useWindowResizing'

/**
 * Refactored Control Panel Events using centralized window registry
 * This replaces the fragile CSS selector-based approach with a robust registry pattern
 */
export function useControlPanelEvents(
  store: any,
  mlEyeTracking: any,
  windowManager: any,
  wakeWordDetection: any,
  providedStateRefs: any
) {
  // Get state management (use provided refs if available, otherwise create new)
  const stateRefs = providedStateRefs || useControlPanelState()
  const windowRegistry = useWindowRegistry()
  const { resizeWindow } = useWindowResizing()

  // Extract reactive refs for easier access
  const {
    showAIModelsWindow,
    showChatWindow,
    showConversationalWindow
  } = stateRefs

  // Window control handlers
  const closeAllWindows = async () => {
    // Close all registered windows via their close handlers
    const allWindows = windowRegistry.getAllWindows()
    for (const window of allWindows) {
      if (window.config.closeHandler) {
        window.config.closeHandler()
      }
    }
    await stateRefs.closeAllWindows()
  }

  // AI Models window handlers - using centralized state management
  const toggleAIModelsWindow = async (event: Event) => {
    event.stopPropagation()
    
    try {
      await stateRefs.toggleWindow('aiModels')
      
      // Resize window based on current state
      if (showAIModelsWindow.value) {
        console.log('⚙️ AI Models window opened - resizing window')
        await resizeWindow(showChatWindow.value, false, showAIModelsWindow.value, showConversationalWindow.value, false)
      } else {
        console.log('⚙️ AI Models window closed - resizing to base')
        await resizeWindow(false, false, false, false, false)
      }
    } catch (error) {
      console.error('❌ Failed to toggle AI models window:', error)
    }
  }

  const closeAIModelsWindow = async () => {
    try {
      await stateRefs.closeAllWindows()
      await resizeWindow(false, false, false, false, false)
      console.log('⚙️ AI Models window closed')
    } catch (error) {
      console.error('❌ Failed to close AI models window:', error)
    }
  }

  // Chat window handlers - using centralized state management
  const toggleChatWindow = async (event: Event) => {
    event.stopPropagation()
    
    try {
      await stateRefs.toggleWindow('chat')
      
      // Resize window based on current state
      if (showChatWindow.value) {
        console.log('💬 Chat window opened - resizing window')
        await resizeWindow(showChatWindow.value, false, showAIModelsWindow.value, showConversationalWindow.value, false)
      } else {
        console.log('💬 Chat window closed - resizing to base')
        await resizeWindow(false, false, false, false, false)
      }
    } catch (error) {
      console.error('❌ Failed to toggle chat window:', error)
    }
  }

  const closeChatWindow = async () => {
    try {
      await stateRefs.closeAllWindows()
      await resizeWindow(false, false, false, false, false)
      console.log('💬 Chat window closed')
    } catch (error) {
      console.error('❌ Failed to close chat window:', error)
    }
  }

  // Conversational window handlers - using centralized state management
  const toggleConversationalWindow = async (event: Event) => {
    event.stopPropagation()
    
    try {
      await stateRefs.toggleWindow('conversational')
      
      // Resize window based on current state
      if (showConversationalWindow.value) {
        console.log('🎤 Conversational window opened - resizing window')
        await resizeWindow(showChatWindow.value, false, showAIModelsWindow.value, showConversationalWindow.value, false)
      } else {
        console.log('🎤 Conversational window closed - resizing to base')
        await resizeWindow(false, false, false, false, false)
      }
    } catch (error) {
      console.error('❌ Failed to toggle conversational window:', error)
    }
  }

  const closeConversationalWindow = async () => {
    try {
      await stateRefs.closeAllWindows()
      await resizeWindow(false, false, false, false, false)
      console.log('🎤 Conversational window closed')
    } catch (error) {
      console.error('❌ Failed to close conversational window:', error)
    }
  }

  // Drag handling - minimal implementation to avoid interfering with Tauri's native drag
  // The data-tauri-drag-region attribute handles the actual dragging
  const handleDragStart = async (event: Event) => {
    // DO NOT call event.stopPropagation() or event.preventDefault()
    // This would prevent Tauri's native drag region from working on macOS
    try {
      console.log('🖱️ Starting window drag')
      // The actual drag implementation would go here
      // This is placeholder for the existing drag functionality
    } catch (error) {
      console.error('❌ Failed to start window drag:', error)
    }
  }

  const handleDragEnd = async () => {
    try {
      console.log('🖱️ Ending window drag')
      // The actual drag end implementation would go here
    } catch (error) {
      console.error('❌ Failed to end window drag:', error)
    }
  }

  // Window management helpers
  const registerWindow = (
    id: string,
    element: HTMLElement,
    closeHandler: () => void,
    options: {
      closeOnClickOutside?: boolean
      isModal?: boolean
      priority?: number
    } = {}
  ) => {
    return windowRegistry.register(id, element, {
      closeHandler,
      closeOnClickOutside: options.closeOnClickOutside ?? true,
      isModal: options.isModal ?? false,
      priority: options.priority ?? 100
    })
  }

  const unregisterWindow = (id: string) => {
    return windowRegistry.unregister(id)
  }

  // Focus management
  const bringWindowToFront = (windowId: string) => {
    return windowRegistry.bringToFront(windowId)
  }

  // State helpers
  const isAnyWindowOpen = () => {
    return showAIModelsWindow.value || 
           showChatWindow.value || 
           showConversationalWindow.value
  }

  const getOpenWindows = () => {
    const openWindows: string[] = []
    if (showAIModelsWindow.value) openWindows.push('ai-models-window')
    if (showChatWindow.value) openWindows.push('chat-window')
    if (showConversationalWindow.value) openWindows.push('conversational-window')
    return openWindows
  }

  // Registry debugging helpers
  const getRegistryInfo = () => {
    const allWindows = windowRegistry.getAllWindows()
    return {
      windowCount: allWindows.length,
      windowIds: allWindows.map(w => w.id),
      hasModals: allWindows.some(w => w.config.isModal)
    }
  }

  // Close specific windows by IDs
  const closeSpecificWindows = async (windowIds: string[]) => {
    const allWindows = windowRegistry.getAllWindows()
    for (const windowId of windowIds) {
      const window = allWindows.find(w => w.id === windowId)
      if (window && window.config.closeHandler) {
        window.config.closeHandler()
      }
    }
    // Also update state for known windows
    for (const windowId of windowIds) {
      if (windowId === 'aiModels' || windowId === 'ai-models-window') {
        showAIModelsWindow.value = false
      } else if (windowId === 'chat' || windowId === 'chat-window') {
        showChatWindow.value = false
      } else if (windowId === 'conversational' || windowId === 'conversational-window') {
        showConversationalWindow.value = false
      }
    }
  }

  // Additional methods for backward compatibility
  const openChatWindow = async () => {
    showChatWindow.value = true
    try {
      await resizeWindow(false, true, false, false, false)
    } catch (error) {
      console.error('❌ Failed to resize window for chat:', error)
    }
  }

  const toggleMLEyeTrackingWithMovement = () => {
    if (mlEyeTracking) {
      mlEyeTracking.toggleMLEyeTrackingWithMovement()
    }
  }

  const handleKeydown = async (event: KeyboardEvent) => {
    // Handle keyboard shortcuts
    if (event.key === 'Escape') {
      await closeAllWindows()
    }
  }

  const handleClickOutside = async (event: Event) => {
    // Let the window registry handle click-outside detection automatically
    // This prevents duplicate detection and conflicts
    // The registry will call individual window close handlers when needed
  }

  return {
    // Window toggle handlers (for buttons)
    toggleAIModelsWindow,
    toggleChatWindow,
    toggleConversationalWindow,
    
    // Window close handlers (for close buttons and registry)
    closeAIModelsWindow,
    closeChatWindow,
    closeConversationalWindow,
    closeAllWindows,
    closeSpecificWindows,
    
    // Additional window methods for backward compatibility
    openChatWindow,
    
    // Drag handlers
    handleDragStart,
    handleDragEnd,
    
    // ML Eye tracking and event handlers
    toggleMLEyeTrackingWithMovement,
    handleKeydown,
    handleClickOutside,
    
    // Window registry integration
    registerWindow,
    unregisterWindow,
    bringWindowToFront,
    
    // State helpers
    isAnyWindowOpen,
    getOpenWindows,
    
    // Registry debugging
    getRegistryInfo,
    
    // Direct access to registry for advanced use cases
    windowRegistry,
    
    // Legacy state refs (for backward compatibility during migration)
    showAIModelsWindow,
    showChatWindow,
    showConversationalWindow
  }
}