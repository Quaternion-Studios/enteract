import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Mock Tauri APIs before importing the composable
const mockInvoke = vi.fn()
const mockListen = vi.fn()
const mockUnlisten = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args)
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args)
}))

// Import after mocking
import { useTransparentClickthrough } from '../../composables/useTransparentClickthrough'

// Helper to create mock DOM elements
const createMockElement = (tagName: string, options: {
  className?: string
  role?: string
  onclick?: boolean
  draggable?: boolean
  dataTauriDragRegion?: boolean
} = {}): Element => {
  const element = document.createElement(tagName)
  if (options.className) element.className = options.className
  if (options.role) element.setAttribute('role', options.role)
  if (options.onclick) element.setAttribute('onclick', 'handler()')
  if (options.draggable) element.setAttribute('draggable', 'true')
  if (options.dataTauriDragRegion) element.setAttribute('data-tauri-drag-region', '')
  return element
}

describe('useTransparentClickthrough', () => {
  let mousePositionHandler: ((event: { payload: { x: number; y: number; isOverWindow: boolean } }) => void) | null = null

  beforeEach(() => {
    vi.clearAllMocks()
    mousePositionHandler = null

    // Setup mock for listen to capture the handler
    mockListen.mockImplementation(async (eventName: string, handler: (event: unknown) => void) => {
      if (eventName === 'mouse-position') {
        mousePositionHandler = handler as typeof mousePositionHandler
      }
      return mockUnlisten
    })

    // Setup mock for invoke to resolve successfully
    mockInvoke.mockResolvedValue(undefined)

    // Mock console methods to suppress output during tests
    vi.spyOn(console, 'log').mockImplementation(() => {})
    vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('Initialization', () => {
    it('sets initial mouse passthrough to true on initialize', async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()

      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('listens for mouse-position events', async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()

      expect(mockListen).toHaveBeenCalledWith('mouse-position', expect.any(Function))
    })

    it('starts mouse tracking via Tauri invoke', async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()

      expect(mockInvoke).toHaveBeenCalledWith('start_mouse_tracking')
    })

    it('handles mouse tracking start failure gracefully', async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'start_mouse_tracking') {
          throw new Error('Failed to start tracking')
        }
        return undefined
      })

      const { initialize } = useTransparentClickthrough()
      await initialize()

      expect(console.error).toHaveBeenCalledWith('Failed to start mouse tracking:', expect.any(Error))
    })
  })

  describe('Cleanup', () => {
    it('calls unlisten function on cleanup', async () => {
      const { initialize, cleanup } = useTransparentClickthrough()
      await initialize()
      await cleanup()

      expect(mockUnlisten).toHaveBeenCalled()
    })

    it('stops mouse tracking via Tauri invoke', async () => {
      const { initialize, cleanup } = useTransparentClickthrough()
      await initialize()
      await cleanup()

      expect(mockInvoke).toHaveBeenCalledWith('stop_mouse_tracking')
    })

    it('handles stop mouse tracking failure gracefully', async () => {
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'stop_mouse_tracking') {
          throw new Error('Failed to stop tracking')
        }
        return undefined
      })

      const { initialize, cleanup } = useTransparentClickthrough()
      await initialize()
      await cleanup()

      expect(console.error).toHaveBeenCalledWith('Failed to stop mouse tracking:', expect.any(Error))
    })

    it('handles cleanup when not initialized', async () => {
      const { cleanup } = useTransparentClickthrough()

      // Should not throw
      await expect(cleanup()).resolves.toBeUndefined()
    })
  })

  describe('Mouse Position Handling', () => {
    beforeEach(async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()
      mockInvoke.mockClear()
    })

    it('enables passthrough when mouse is outside window after being over interactive element', async () => {
      expect(mousePositionHandler).not.toBeNull()

      // First, move over an interactive element to change state
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(button)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

      mockInvoke.mockClear()

      // Now move outside window - should re-enable passthrough
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: false } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('enables passthrough when no element found after being over interactive element', async () => {
      // First, move over an interactive element to change state
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValueOnce(button).mockReturnValueOnce(null)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

      mockInvoke.mockClear()

      // Now move to position with no element
      await mousePositionHandler!({ payload: { x: 200, y: 200, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('does not call setMousePassthrough when already in passthrough state and mouse is outside', async () => {
      // Initial state is passthrough=true, so moving outside should not trigger a call
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: false } })
      expect(mockInvoke).not.toHaveBeenCalled()
    })

    it('uses correct coordinates from Rust backend', async () => {
      const elementFromPointSpy = vi.spyOn(document, 'elementFromPoint').mockReturnValue(document.body)

      await mousePositionHandler!({ payload: { x: 250, y: 350, isOverWindow: true } })

      expect(elementFromPointSpy).toHaveBeenCalledWith(250, 350)
    })
  })

  describe('Transparent Area Detection', () => {
    beforeEach(async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()
      mockInvoke.mockClear()
    })

    it('enables passthrough when over html element after interactive element', async () => {
      // First, move over an interactive element to change state
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValueOnce(button).mockReturnValueOnce(document.documentElement)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

      mockInvoke.mockClear()

      // Now move over html element (transparent area)
      await mousePositionHandler!({ payload: { x: 200, y: 200, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('enables passthrough when over body element after interactive element', async () => {
      // First, move over an interactive element to change state
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValueOnce(button).mockReturnValueOnce(document.body)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

      mockInvoke.mockClear()

      // Now move over body element (transparent area)
      await mousePositionHandler!({ payload: { x: 200, y: 200, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('does not call setMousePassthrough when already in passthrough state over html', async () => {
      // Initial state is passthrough=true, so hovering over transparent area should not trigger a call
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(document.documentElement)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).not.toHaveBeenCalled()
    })

    it('does not call setMousePassthrough when already in passthrough state over body', async () => {
      // Initial state is passthrough=true, so hovering over transparent area should not trigger a call
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(document.body)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).not.toHaveBeenCalled()
    })
  })

  describe('Interactive Element Detection', () => {
    beforeEach(async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()
      mockInvoke.mockClear()
    })

    describe('Standard HTML Elements', () => {
      it('disables passthrough when over button element', async () => {
        const button = createMockElement('button')
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(button)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over input element', async () => {
        const input = createMockElement('input')
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(input)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over select element', async () => {
        const select = createMockElement('select')
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(select)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over textarea element', async () => {
        const textarea = createMockElement('textarea')
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(textarea)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over anchor element', async () => {
        const anchor = createMockElement('a')
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(anchor)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })
    })

    describe('ARIA Role Elements', () => {
      it('disables passthrough when over element with role="button"', async () => {
        const div = createMockElement('div', { role: 'button' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over element with role="link"', async () => {
        const span = createMockElement('span', { role: 'link' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(span)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over element with role="menuitem"', async () => {
        const li = createMockElement('li', { role: 'menuitem' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(li)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })
    })

    describe('Onclick Elements', () => {
      it('disables passthrough when over element with onclick attribute', async () => {
        const div = createMockElement('div', { onclick: true })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })
    })

    describe('UI Component Classes', () => {
      it('disables passthrough when over control-panel class', async () => {
        const div = createMockElement('div', { className: 'control-panel' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over control-panel-glass-bar class', async () => {
        const div = createMockElement('div', { className: 'control-panel-glass-bar' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over chat-sidebar class', async () => {
        const div = createMockElement('div', { className: 'chat-sidebar' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over drawer class', async () => {
        const div = createMockElement('div', { className: 'drawer' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over modal class', async () => {
        const div = createMockElement('div', { className: 'modal' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over menu class', async () => {
        const div = createMockElement('div', { className: 'menu' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over dropdown class', async () => {
        const div = createMockElement('div', { className: 'dropdown' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over card class', async () => {
        const div = createMockElement('div', { className: 'card' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over app-layout class', async () => {
        const div = createMockElement('div', { className: 'app-layout' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })
    })

    describe('Wildcard Class Patterns', () => {
      it('disables passthrough when over class containing "panel"', async () => {
        const div = createMockElement('div', { className: 'settings-panel-main' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over class containing "sidebar"', async () => {
        const div = createMockElement('div', { className: 'nav-sidebar-left' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over class containing "control"', async () => {
        const div = createMockElement('div', { className: 'volume-control' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over class containing "window"', async () => {
        const div = createMockElement('div', { className: 'chat-window-container' })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })
    })

    describe('Draggable Elements', () => {
      it('disables passthrough when over element with data-tauri-drag-region', async () => {
        const div = createMockElement('div', { dataTauriDragRegion: true })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })

      it('disables passthrough when over element with draggable="true"', async () => {
        const div = createMockElement('div', { draggable: true })
        vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
      })
    })

    describe('Parent Element Detection', () => {
      it('disables passthrough when parent element is interactive', async () => {
        const button = createMockElement('button')
        const span = document.createElement('span')
        span.textContent = 'Click me'
        button.appendChild(span)
        document.body.appendChild(button)

        vi.spyOn(document, 'elementFromPoint').mockReturnValue(span)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

        document.body.removeChild(button)
      })

      it('disables passthrough when ancestor has interactive class', async () => {
        const panel = createMockElement('div', { className: 'control-panel' })
        const nested = document.createElement('div')
        const inner = document.createElement('span')
        nested.appendChild(inner)
        panel.appendChild(nested)
        document.body.appendChild(panel)

        vi.spyOn(document, 'elementFromPoint').mockReturnValue(inner)

        await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

        expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

        document.body.removeChild(panel)
      })
    })
  })

  describe('State Change Optimization', () => {
    beforeEach(async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()
      mockInvoke.mockClear()
    })

    it('does not call setMousePassthrough when state has not changed', async () => {
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(button)

      // First call - should set passthrough
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledTimes(1)

      mockInvoke.mockClear()

      // Second call over same type of element - should not call again
      await mousePositionHandler!({ payload: { x: 150, y: 150, isOverWindow: true } })
      expect(mockInvoke).not.toHaveBeenCalled()
    })

    it('calls setMousePassthrough when transitioning from interactive to transparent', async () => {
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint')
        .mockReturnValueOnce(button)
        .mockReturnValueOnce(document.body)

      // First call - over button
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })

      mockInvoke.mockClear()

      // Second call - over body (transparent)
      await mousePositionHandler!({ payload: { x: 200, y: 200, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('calls setMousePassthrough when transitioning from transparent to interactive', async () => {
      vi.spyOn(document, 'elementFromPoint')
        .mockReturnValueOnce(document.body)
        .mockReturnValueOnce(createMockElement('button'))

      // First call - over body (transparent)
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      // Initial state is passthrough ON, so no change needed for transparent area
      expect(mockInvoke).not.toHaveBeenCalled()

      mockInvoke.mockClear()

      // Second call - over button
      await mousePositionHandler!({ payload: { x: 200, y: 200, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
    })
  })

  describe('setMousePassthrough', () => {
    it('calls Tauri invoke with correct parameters for passthrough ON', async () => {
      const { setMousePassthrough } = useTransparentClickthrough()

      await setMousePassthrough(true)

      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: true })
    })

    it('calls Tauri invoke with correct parameters for passthrough OFF', async () => {
      const { setMousePassthrough } = useTransparentClickthrough()

      await setMousePassthrough(false)

      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
    })

    it('handles setMousePassthrough errors gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Tauri invoke failed'))

      const { setMousePassthrough } = useTransparentClickthrough()
      await setMousePassthrough(true)

      expect(console.error).toHaveBeenCalledWith('Failed to set mouse passthrough:', expect.any(Error))
    })
  })

  describe('Non-Interactive Elements', () => {
    beforeEach(async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()
      mockInvoke.mockClear()
    })

    it('enables passthrough when over plain div without interactive classes', async () => {
      const div = createMockElement('div', { className: 'some-random-class' })
      document.body.appendChild(div)
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(div)

      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

      // Should remain passthrough (initial state), so no call needed
      expect(mockInvoke).not.toHaveBeenCalled()

      document.body.removeChild(div)
    })

    it('enables passthrough when over plain span element', async () => {
      const span = document.createElement('span')
      span.textContent = 'Just text'
      document.body.appendChild(span)
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(span)

      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

      // Should remain passthrough (initial state), so no call needed
      expect(mockInvoke).not.toHaveBeenCalled()

      document.body.removeChild(span)
    })

    it('enables passthrough when over paragraph element', async () => {
      const p = document.createElement('p')
      p.textContent = 'Paragraph text'
      document.body.appendChild(p)
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(p)

      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })

      // Should remain passthrough (initial state), so no call needed
      expect(mockInvoke).not.toHaveBeenCalled()

      document.body.removeChild(p)
    })
  })

  describe('Edge Cases', () => {
    beforeEach(async () => {
      const { initialize } = useTransparentClickthrough()
      await initialize()
      mockInvoke.mockClear()
    })

    it('handles rapid mouse position updates', async () => {
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(button)

      // Simulate rapid updates
      const promises = []
      for (let i = 0; i < 10; i++) {
        promises.push(mousePositionHandler!({ payload: { x: 100 + i, y: 100 + i, isOverWindow: true } }))
      }
      await Promise.all(promises)

      // Should only call once since state doesn't change
      expect(mockInvoke).toHaveBeenCalledTimes(1)
    })

    it('handles window boundary coordinates', async () => {
      const elementFromPointSpy = vi.spyOn(document, 'elementFromPoint').mockReturnValue(document.body)

      // Test edge coordinates
      await mousePositionHandler!({ payload: { x: 0, y: 0, isOverWindow: true } })
      expect(elementFromPointSpy).toHaveBeenCalledWith(0, 0)

      await mousePositionHandler!({ payload: { x: -1, y: -1, isOverWindow: true } })
      expect(elementFromPointSpy).toHaveBeenCalledWith(-1, -1)
    })

    it('handles transition from outside to inside window', async () => {
      const button = createMockElement('button')
      vi.spyOn(document, 'elementFromPoint').mockReturnValue(button)

      // First outside window
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: false } })
      expect(mockInvoke).not.toHaveBeenCalled() // Already in passthrough state

      mockInvoke.mockClear()

      // Then inside window over button
      await mousePositionHandler!({ payload: { x: 100, y: 100, isOverWindow: true } })
      expect(mockInvoke).toHaveBeenCalledWith('set_mouse_passthrough', { passthrough: false })
    })
  })

  describe('Return Value', () => {
    it('returns initialize, cleanup, and setMousePassthrough functions', () => {
      const result = useTransparentClickthrough()

      expect(result).toHaveProperty('initialize')
      expect(result).toHaveProperty('cleanup')
      expect(result).toHaveProperty('setMousePassthrough')
      expect(typeof result.initialize).toBe('function')
      expect(typeof result.cleanup).toBe('function')
      expect(typeof result.setMousePassthrough).toBe('function')
    })
  })
})
