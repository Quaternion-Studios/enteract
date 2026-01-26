/**
 * Transparent Click-Through Composable
 *
 * Enables click-through on transparent window areas while keeping UI interactive.
 *
 * Strategy:
 * - Tracks mouse position globally
 * - Detects if cursor is over an interactive element
 * - Toggles window's setIgnoresMouseEvents based on cursor location:
 *   * Over interactive element → window captures clicks (passthrough OFF)
 *   * Over transparent/empty area → clicks pass through (passthrough ON)
 */

import { onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Selectors for interactive elements that should capture mouse events
const INTERACTIVE_SELECTORS = [
  'button',
  'input',
  'select',
  'textarea',
  'a',
  '[role="button"]',
  '[role="link"]',
  '[role="menuitem"]',
  '[onclick]',
  '.control-panel',
  '.chat-sidebar',
  '.drawer',
  '.modal',
  '.menu',
  '.dropdown',
  '.card',
  '[class*="panel"]',
  '[class*="sidebar"]',
  '[class*="control"]',
  '[draggable="true"]'
].join(', ')

export function useTransparentClickthrough() {
  let isOverInteractiveElement = false
  let mouseMoveThrottleTimer: number | null = null

  const handleMouseMove = async (event: MouseEvent) => {
    // Throttle checks to avoid excessive Tauri calls (every 50ms max)
    if (mouseMoveThrottleTimer !== null) {
      return
    }

    mouseMoveThrottleTimer = window.setTimeout(() => {
      mouseMoveThrottleTimer = null
    }, 50)

    // Get element at cursor position
    const element = document.elementFromPoint(event.clientX, event.clientY)

    if (!element) {
      // No element found - enable passthrough
      if (!isOverInteractiveElement) {
        return // Already in passthrough mode
      }
      isOverInteractiveElement = false
      await setMousePassthrough(true)
      return
    }

    // Check if element or any parent is interactive
    const isInteractive = element.matches(INTERACTIVE_SELECTORS) ||
                         element.closest(INTERACTIVE_SELECTORS) !== null

    // Only update if state changed (avoid redundant calls)
    if (isInteractive !== isOverInteractiveElement) {
      isOverInteractiveElement = isInteractive
      // passthrough = NOT interactive (inverted logic)
      await setMousePassthrough(!isInteractive)
    }
  }

  const setMousePassthrough = async (passthrough: boolean) => {
    try {
      await invoke('set_mouse_passthrough', { passthrough })
      console.log(`🖱️  Mouse passthrough: ${passthrough ? 'ON (clicks pass through)' : 'OFF (window captures clicks)'}`)
    } catch (error) {
      console.error('Failed to set mouse passthrough:', error)
    }
  }

  const initialize = () => {
    // Start with passthrough enabled (clicks pass through by default)
    setMousePassthrough(true)

    // Add global mouse move listener
    document.addEventListener('mousemove', handleMouseMove, { passive: true })

    console.log('✅ Transparent click-through initialized')
  }

  const cleanup = () => {
    document.removeEventListener('mousemove', handleMouseMove)

    if (mouseMoveThrottleTimer !== null) {
      clearTimeout(mouseMoveThrottleTimer)
    }

    console.log('🧹 Transparent click-through cleaned up')
  }

  onMounted(() => {
    initialize()
  })

  onUnmounted(() => {
    cleanup()
  })

  return {
    initialize,
    cleanup,
    setMousePassthrough
  }
}
