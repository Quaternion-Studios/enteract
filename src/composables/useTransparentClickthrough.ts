/**
 * Transparent Click-Through Composable
 *
 * Enables click-through on transparent window areas while keeping UI interactive.
 *
 * Strategy:
 * - Uses Rust backend to poll global mouse position (works even when window ignores events)
 * - Receives mouse position events from backend via Tauri event system
 * - Detects if cursor is over an interactive element
 * - Toggles window's setIgnoresMouseEvents based on cursor location:
 *   * Over interactive element → window captures clicks (passthrough OFF)
 *   * Over transparent/empty area → clicks pass through (passthrough ON)
 */

import { onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

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
  '.control-panel-glass-bar',
  '.control-panel-section',
  '.chat-sidebar',
  '.drawer',
  '.modal',
  '.menu',
  '.dropdown',
  '.card',
  '.app-layout',
  '[class*="panel"]',
  '[class*="sidebar"]',
  '[class*="control"]',
  '[class*="window"]',
  '[data-tauri-drag-region]',
  '[draggable="true"]'
].join(', ')

export function useTransparentClickthrough() {
  let isOverInteractiveElement = false
  let lastElement: Element | null = null
  let unlistenFn: UnlistenFn | null = null

  const handleMousePosition = async (event: any) => {
    const { x, y, isOverWindow } = event.payload

    if (!isOverWindow) {
      // Mouse is outside window - enable passthrough
      if (isOverInteractiveElement) {
        isOverInteractiveElement = false
        await setMousePassthrough(true)
      }
      return
    }

    // Get element at cursor position (coordinates from Rust are already in web/DOM coordinates)
    const element = document.elementFromPoint(x, y)

    if (!element) {
      // No element found - enable passthrough
      if (isOverInteractiveElement) {
        isOverInteractiveElement = false
        await setMousePassthrough(true)
      }
      return
    }

    // If hovering over html or body only, that means transparent background
    const tagName = element.tagName.toLowerCase()
    const isTransparentArea = tagName === 'html' || tagName === 'body'

    // Check if element or any parent is interactive
    const isInteractive = !isTransparentArea && (
      element.matches(INTERACTIVE_SELECTORS) ||
      element.closest(INTERACTIVE_SELECTORS) !== null
    )

    // Debug logging when element changes
    if (element !== lastElement) {
      console.log(`🎯 Element under cursor: <${tagName}> class="${element.className}" interactive=${isInteractive}`)
      lastElement = element
    }

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

  const initialize = async () => {
    // Start with passthrough enabled (clicks pass through by default)
    await setMousePassthrough(true)

    // Listen for mouse position events from Rust backend
    unlistenFn = await listen('mouse-position', handleMousePosition)

    // Start the Rust mouse tracking thread
    try {
      await invoke('start_mouse_tracking')
      console.log('✅ Transparent click-through initialized with global mouse tracking')
    } catch (error) {
      console.error('Failed to start mouse tracking:', error)
    }
  }

  const cleanup = async () => {
    // Stop listening to events
    if (unlistenFn) {
      unlistenFn()
      unlistenFn = null
    }

    // Stop the Rust tracking thread
    try {
      await invoke('stop_mouse_tracking')
    } catch (error) {
      console.error('Failed to stop mouse tracking:', error)
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
