import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, VueWrapper } from '@vue/test-utils'
import { ref } from 'vue'
import ChatSidebarAdapter from '@/components/core/ChatSidebarAdapter.vue'
import { createMockChatManagement } from '../__mocks__/composables'

// Mock data
const mockChatSessions = ref([
  { id: 'chat-1', title: 'First Chat', updatedAt: '2024-01-15T10:00:00Z', history: [] },
  { id: 'chat-2', title: 'Second Chat', updatedAt: '2024-01-15T11:00:00Z', history: [{ role: 'user', content: 'Hello' }] },
  { id: 'chat-3', title: 'Third Chat', updatedAt: '2024-01-15T09:00:00Z', history: [] },
])

const mockCurrentChatId = ref('chat-1')

// Create mock functions that we can track
const mockCreateNewChat = vi.fn()
const mockSwitchChat = vi.fn()
const mockDeleteChat = vi.fn()
const mockRenameChat = vi.fn()
const mockClearChat = vi.fn()

// Mock useChatManagement composable
vi.mock('@/composables/useChatManagement', () => ({
  useChatManagement: () => ({
    ...createMockChatManagement(),
    chatSessions: mockChatSessions,
    currentChatId: mockCurrentChatId,
    createNewChat: mockCreateNewChat,
    switchChat: mockSwitchChat,
    deleteChat: mockDeleteChat,
    renameChat: mockRenameChat,
    clearChat: mockClearChat,
  }),
}))

// Mock GenericSidebar component to capture props and events
vi.mock('@/components/core/GenericSidebar.vue', () => ({
  default: {
    name: 'GenericSidebar',
    props: [
      'items', 'currentItemId', 'isLoading', 'title', 'icon',
      'emptyMessage', 'emptySubMessage', 'theme', 'displayMode',
      'showNewButton', 'showDeleteButton', 'showRenameButton',
      'showClearAllButton', 'showTimestamps', 'showMetadata'
    ],
    emits: ['close', 'new-item', 'select-item', 'delete-item', 'rename-item', 'clear-all'],
    template: `
      <div data-testid="generic-sidebar">
        <div data-testid="sidebar-title">{{ title }}</div>
        <div data-testid="sidebar-items-count">{{ items?.length || 0 }}</div>
        <div data-testid="sidebar-current-id">{{ currentItemId }}</div>
        <div data-testid="sidebar-theme">{{ theme }}</div>
        <div data-testid="sidebar-display-mode">{{ displayMode }}</div>
        <button data-testid="close-btn" @click="$emit('close')">Close</button>
        <button data-testid="new-item-btn" @click="$emit('new-item')">New</button>
        <button data-testid="select-item-btn" @click="$emit('select-item', 'chat-2')">Select</button>
        <button data-testid="delete-current-btn" @click="$emit('delete-item', 'chat-1')">Delete Current</button>
        <button data-testid="delete-other-btn" @click="$emit('delete-item', 'chat-2')">Delete Other</button>
        <button data-testid="rename-btn" @click="$emit('rename-item', 'chat-1', 'Renamed Chat')">Rename</button>
        <button data-testid="clear-all-btn" @click="$emit('clear-all')">Clear All</button>
      </div>
    `,
  },
}))

describe('ChatSidebarAdapter', () => {
  let wrapper: VueWrapper<any>
  const defaultProps = {
    selectedModel: 'test-model',
    isOpen: true,
  }

  beforeEach(() => {
    vi.clearAllMocks()
    mockCurrentChatId.value = 'chat-1'
    mockChatSessions.value = [
      { id: 'chat-1', title: 'First Chat', updatedAt: '2024-01-15T10:00:00Z', history: [] },
      { id: 'chat-2', title: 'Second Chat', updatedAt: '2024-01-15T11:00:00Z', history: [{ role: 'user', content: 'Hello' }] },
      { id: 'chat-3', title: 'Third Chat', updatedAt: '2024-01-15T09:00:00Z', history: [] },
    ]
  })

  const createWrapper = (props = {}) => {
    wrapper = mount(ChatSidebarAdapter, {
      props: { ...defaultProps, ...props },
    })
    return wrapper
  }

  describe('Rendering', () => {
    it('renders GenericSidebar when isOpen is true', () => {
      createWrapper()
      expect(wrapper.find('[data-testid="generic-sidebar"]').exists()).toBe(true)
    })

    it('does not render GenericSidebar when isOpen is false', () => {
      createWrapper({ isOpen: false })
      expect(wrapper.find('[data-testid="generic-sidebar"]').exists()).toBe(false)
    })

    it('shows correct title', () => {
      createWrapper()
      expect(wrapper.find('[data-testid="sidebar-title"]').text()).toBe('Chat Sessions')
    })

    it('passes correct theme to GenericSidebar', () => {
      createWrapper()
      expect(wrapper.find('[data-testid="sidebar-theme"]').text()).toBe('blue')
    })

    it('passes correct displayMode to GenericSidebar', () => {
      createWrapper()
      expect(wrapper.find('[data-testid="sidebar-display-mode"]').text()).toBe('overlay')
    })
  })

  describe('Props Passing', () => {
    it('transforms chatSessions to sidebar items', () => {
      createWrapper()
      const itemsCount = wrapper.find('[data-testid="sidebar-items-count"]').text()
      expect(itemsCount).toBe('3')
    })

    it('passes currentChatId to GenericSidebar', () => {
      createWrapper()
      expect(wrapper.find('[data-testid="sidebar-current-id"]').text()).toBe('chat-1')
    })

    it('updates currentItemId when currentChatId changes', async () => {
      createWrapper()
      mockCurrentChatId.value = 'chat-2'
      await wrapper.vm.$nextTick()
      expect(wrapper.find('[data-testid="sidebar-current-id"]').text()).toBe('chat-2')
    })
  })

  describe('Event Handling - New Item', () => {
    it('calls createNewChat when new-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="new-item-btn"]').trigger('click')
      expect(mockCreateNewChat).toHaveBeenCalledTimes(1)
    })

    it('emits open-chat-window when new-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="new-item-btn"]').trigger('click')
      expect(wrapper.emitted('open-chat-window')).toBeTruthy()
      expect(wrapper.emitted('open-chat-window')).toHaveLength(1)
    })

    it('emits close when new-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="new-item-btn"]').trigger('click')
      expect(wrapper.emitted('close')).toBeTruthy()
      expect(wrapper.emitted('close')).toHaveLength(1)
    })
  })

  describe('Event Handling - Select Item', () => {
    it('calls switchChat with correct id when select-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="select-item-btn"]').trigger('click')
      expect(mockSwitchChat).toHaveBeenCalledWith('chat-2')
    })

    it('emits close when select-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="select-item-btn"]').trigger('click')
      expect(wrapper.emitted('close')).toBeTruthy()
    })
  })

  describe('Event Handling - Delete Item', () => {
    it('calls deleteChat with correct id when delete-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="delete-other-btn"]').trigger('click')
      expect(mockDeleteChat).toHaveBeenCalledWith('chat-2')
    })

    it('emits open-chat-window when deleting the current chat', async () => {
      createWrapper()
      await wrapper.find('[data-testid="delete-current-btn"]').trigger('click')
      expect(mockDeleteChat).toHaveBeenCalledWith('chat-1')
      expect(wrapper.emitted('open-chat-window')).toBeTruthy()
    })

    it('does not emit open-chat-window when deleting a non-current chat', async () => {
      createWrapper()
      await wrapper.find('[data-testid="delete-other-btn"]').trigger('click')
      expect(mockDeleteChat).toHaveBeenCalledWith('chat-2')
      expect(wrapper.emitted('open-chat-window')).toBeFalsy()
    })
  })

  describe('Event Handling - Rename Item', () => {
    it('calls renameChat with correct id and new title when rename-item event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="rename-btn"]').trigger('click')
      expect(mockRenameChat).toHaveBeenCalledWith('chat-1', 'Renamed Chat')
    })
  })

  describe('Event Handling - Clear All', () => {
    it('calls clearChat when clear-all event is triggered', async () => {
      createWrapper()
      await wrapper.find('[data-testid="clear-all-btn"]').trigger('click')
      expect(mockClearChat).toHaveBeenCalledTimes(1)
    })
  })

  describe('Event Handling - Close', () => {
    it('emits close when close event is triggered from GenericSidebar', async () => {
      createWrapper()
      await wrapper.find('[data-testid="close-btn"]').trigger('click')
      expect(wrapper.emitted('close')).toBeTruthy()
    })
  })

  describe('Data Transformation', () => {
    it('transforms chat sessions correctly with title', () => {
      createWrapper()
      // The items are transformed via transformChats utility
      // We verify by checking that the sidebar receives the correct count
      expect(wrapper.find('[data-testid="sidebar-items-count"]').text()).toBe('3')
    })

    it('updates items when chatSessions changes', async () => {
      createWrapper()
      expect(wrapper.find('[data-testid="sidebar-items-count"]').text()).toBe('3')

      mockChatSessions.value = [
        { id: 'chat-1', title: 'Only Chat', updatedAt: '2024-01-15T10:00:00Z', history: [] },
      ]
      await wrapper.vm.$nextTick()

      expect(wrapper.find('[data-testid="sidebar-items-count"]').text()).toBe('1')
    })

    it('handles empty chatSessions', async () => {
      mockChatSessions.value = []
      createWrapper()
      expect(wrapper.find('[data-testid="sidebar-items-count"]').text()).toBe('0')
    })
  })

  describe('Edge Cases', () => {
    it('handles null selectedModel', () => {
      createWrapper({ selectedModel: null })
      expect(wrapper.find('[data-testid="generic-sidebar"]').exists()).toBe(true)
    })

    it('handles rapid open/close toggling', async () => {
      createWrapper({ isOpen: true })
      expect(wrapper.find('[data-testid="generic-sidebar"]').exists()).toBe(true)

      await wrapper.setProps({ isOpen: false })
      expect(wrapper.find('[data-testid="generic-sidebar"]').exists()).toBe(false)

      await wrapper.setProps({ isOpen: true })
      expect(wrapper.find('[data-testid="generic-sidebar"]').exists()).toBe(true)
    })

    it('handles multiple consecutive new-item clicks', async () => {
      createWrapper()
      const newBtn = wrapper.find('[data-testid="new-item-btn"]')

      await newBtn.trigger('click')
      await newBtn.trigger('click')
      await newBtn.trigger('click')

      expect(mockCreateNewChat).toHaveBeenCalledTimes(3)
      expect(wrapper.emitted('open-chat-window')).toHaveLength(3)
      expect(wrapper.emitted('close')).toHaveLength(3)
    })

    it('handles deleting all chats one by one', async () => {
      createWrapper()

      // Delete current chat
      await wrapper.find('[data-testid="delete-current-btn"]').trigger('click')
      expect(mockDeleteChat).toHaveBeenCalledWith('chat-1')
      expect(wrapper.emitted('open-chat-window')).toHaveLength(1)

      // Simulate currentChatId changing after delete
      mockCurrentChatId.value = 'chat-2'
      await wrapper.vm.$nextTick()

      // Delete another chat (now the current one)
      await wrapper.find('[data-testid="delete-current-btn"]').trigger('click')
      // This will still try to delete 'chat-1' due to our mock button,
      // but we verify the callback was invoked
      expect(mockDeleteChat).toHaveBeenCalledTimes(2)
    })
  })
})
