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

export const createMockChatManagement = (overrides?: Partial<ReturnType<typeof createMockChatManagement>>) => ({
  chatMessage: ref(''),
  chatHistory: ref([]),
  chatSessions: ref([
    { id: 'chat-1', title: 'First Chat', updatedAt: '2024-01-15T10:00:00Z', history: [] },
    { id: 'chat-2', title: 'Second Chat', updatedAt: '2024-01-15T11:00:00Z', history: [{ role: 'user', content: 'Hello' }] },
  ]),
  currentChatId: ref('chat-1'),
  currentChatHistory: ref([]),
  currentChatSession: ref(null),
  createNewChat: vi.fn(),
  switchChat: vi.fn(),
  deleteChat: vi.fn(),
  renameChat: vi.fn(),
  clearChat: vi.fn(),
  loadAllChats: vi.fn(),
  saveAllChats: vi.fn(),
  fileInput: ref(null),
  renderMarkdown: vi.fn((text: string) => text),
  takeScreenshotAndAnalyze: vi.fn(),
  startDeepResearch: vi.fn(),
  startConversationalAgent: vi.fn(),
  startCodingAgent: vi.fn(),
  startComputerUseAgent: vi.fn(),
  sendMessage: vi.fn(),
  handleChatKeydown: vi.fn(),
  triggerFileUpload: vi.fn(),
  handleFileUpload: vi.fn(),
  estimateTokens: vi.fn(() => 100),
  cancelResponse: vi.fn(),
  ...overrides,
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