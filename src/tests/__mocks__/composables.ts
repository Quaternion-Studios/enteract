import { vi } from 'vitest'
import { ref } from 'vue'

export const createMockWindowManager = () => ({
  initializeWindow: vi.fn(),
  resizeWindow: vi.fn(),
  openWindow: vi.fn(),
  closeWindow: vi.fn(),
  toggleWindow: vi.fn(),
})

export const createMockControlPanelState = () => ({
  isDragging: ref(false),
  dragStartTime: ref(0),
  showChatWindow: ref(false),
  showAIModelsWindow: ref(false),
  showConversationalWindow: ref(false),
  speechError: ref(null),
  compatibilityReport: ref(null),
  isGazeControlActive: ref(false),
  dragIndicatorVisible: ref(false),
  closeAllWindows: vi.fn(),
  openWindow: vi.fn(),
  toggleWindow: vi.fn(),
  getWindowState: vi.fn(),
  hasOpenWindow: vi.fn(() => false),
})

export const createMockSpeechTranscription = () => ({
  initialize: vi.fn().mockResolvedValue(undefined),
  startRecording: vi.fn().mockResolvedValue(undefined),
  stopRecording: vi.fn().mockResolvedValue(undefined),
  isRecording: ref(false),
  isInitialized: ref(true),
  error: ref(null),
  setAutoSendToChat: vi.fn(),
  setContinuousMode: vi.fn(),
})

export const createMockChatManagement = () => ({
  chatMessage: ref(''),
  chatHistory: ref([]),
  createNewChat: vi.fn(),
  switchChat: vi.fn(),
  deleteChat: vi.fn(),
  clearChat: vi.fn(),
  fileInput: ref(null),
  renderMarkdown: vi.fn((text: string) => text),
  takeScreenshotAndAnalyze: vi.fn(),
  startDeepResearch: vi.fn(),
  startConversationalAgent: vi.fn(),
  startCodingAgent: vi.fn(),
  startComputerUseAgent: vi.fn(),
  sendMessage: vi.fn(),
  triggerFileUpload: vi.fn(),
  handleFileUpload: vi.fn(),
  estimateTokens: vi.fn(() => 100),
})

export const createMockMLEyeTracking = () => ({
  isActive: ref(false),
  isInitialized: ref(false),
  error: ref(null),
  currentGaze: ref({ x: 0, y: 0 }),
  initialize: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  cleanup: vi.fn(),
})

export const createMockConversationStore = () => ({
  currentSession: ref(null),
  currentMessages: ref([]),
  sessions: ref([]),
  isAudioLoopbackActive: ref(false),
  createSession: vi.fn(),
  endSession: vi.fn(),
  setAudioLoopbackState: vi.fn(),
  getMessagePersistenceStatus: vi.fn(() => ({
    pendingCount: 0,
    failedCount: 0,
    isSaving: false,
    isOnline: true,
    stats: {
      totalSaved: 0,
      totalFailed: 0,
      lastSaveTime: null
    }
  })),
})

export const createMockWindowRegistry = () => ({
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
})

export const createMockWindowRegistration = () => ({
  registerSelf: vi.fn(),
  unregisterSelf: vi.fn(),
  updateConfig: vi.fn(),
  ...createMockWindowRegistry(),
})