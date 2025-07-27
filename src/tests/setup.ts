import { vi } from 'vitest'
import { config } from '@vue/test-utils'

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-opener', () => ({
  open: vi.fn(),
}))

// Mock DOM APIs that might not be available in test environment
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// Mock speech recognition API
Object.defineProperty(window, 'SpeechRecognition', {
  writable: true,
  value: vi.fn().mockImplementation(() => ({
    start: vi.fn(),
    stop: vi.fn(),
    abort: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    continuous: false,
    interimResults: false,
    lang: 'en-US',
  })),
})

Object.defineProperty(window, 'webkitSpeechRecognition', {
  writable: true,
  value: window.SpeechRecognition,
})

// Mock requestAnimationFrame
global.requestAnimationFrame = vi.fn(cb => window.setTimeout(cb, 16) as unknown as number)
global.cancelAnimationFrame = vi.fn()

// Mock IntersectionObserver
global.IntersectionObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}))

// Mock window registry composables
vi.mock('@/composables/useWindowRegistry', () => ({
  useWindowRegistry: () => ({
    register: vi.fn(),
    unregister: vi.fn(),
    getWindow: vi.fn(),
    getAllWindows: vi.fn(() => []),
    getActiveWindows: vi.fn(() => []),
    isRegistered: vi.fn(() => false),
    isActive: vi.fn(() => false),
    setActive: vi.fn(),
    setInactive: vi.fn(),
    bringToFront: vi.fn(),
    isClickOutside: vi.fn(() => true),
    isClickOutsideAll: vi.fn(() => true),
    cleanup: vi.fn(),
  }),
  useWindowRegistration: () => ({
    registerSelf: vi.fn(),
    unregisterSelf: vi.fn(),
    updateConfig: vi.fn(),
    register: vi.fn(),
    unregister: vi.fn(),
    getWindow: vi.fn(),
    getAllWindows: vi.fn(() => []),
    getActiveWindows: vi.fn(() => []),
    isRegistered: vi.fn(() => false),
    isActive: vi.fn(() => false),
    setActive: vi.fn(),
    setInactive: vi.fn(),
    bringToFront: vi.fn(),
    isClickOutside: vi.fn(() => true),
    isClickOutsideAll: vi.fn(() => true),
    cleanup: vi.fn(),
  }),
}))

// Global test configuration for Vue Test Utils
config.global.stubs = {
  // Stub out icons that might cause issues
  'XMarkIcon': true,
  'MicrophoneIcon': true,
  'SpeakerWaveIcon': true,
  'CommandLineIcon': true,
  'QueueListIcon': true,
  'PencilIcon': true,
  'SparklesIcon': true,
  'RocketLaunchIcon': true,
  'ShieldCheckIcon': true,
  'StopIcon': true,
  'ExclamationTriangleIcon': true,
}