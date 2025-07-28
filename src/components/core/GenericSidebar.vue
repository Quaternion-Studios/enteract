<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  XMarkIcon,
  PlusIcon,
  TrashIcon,
  PencilIcon,
  EllipsisVerticalIcon,
  ClockIcon
} from '@heroicons/vue/24/outline'

// Generic item interface that can represent conversations, chats, etc.
export interface SidebarItem {
  id: string
  title: string
  subtitle?: string
  timestamp: Date | string | number
  metadata?: {
    count?: number
    duration?: string
    preview?: string
    [key: string]: any
  }
  isActive?: boolean
}

export interface Props {
  // Core props
  items: SidebarItem[]
  currentItemId?: string | null
  isLoading?: boolean
  
  // Display props
  title?: string
  icon?: any
  emptyMessage?: string
  emptySubMessage?: string
  
  // Style props
  theme?: 'purple' | 'blue'
  displayMode?: 'inline' | 'overlay'
  width?: string
  
  // Feature flags
  showNewButton?: boolean
  showDeleteButton?: boolean
  showRenameButton?: boolean
  showClearAllButton?: boolean
  showTimestamps?: boolean
  showMetadata?: boolean
}

export interface Emits {
  (e: 'close'): void
  (e: 'new-item'): void
  (e: 'select-item', id: string): void
  (e: 'delete-item', id: string): void
  (e: 'rename-item', id: string, newTitle: string): void
  (e: 'clear-all'): void
}

const props = withDefaults(defineProps<Props>(), {
  title: 'Items',
  emptyMessage: 'No items yet',
  theme: 'blue',
  displayMode: 'inline',
  width: 'w-80',
  showNewButton: true,
  showDeleteButton: true,
  showRenameButton: true,
  showClearAllButton: false,
  showTimestamps: true,
  showMetadata: true
})

const emit = defineEmits<Emits>()

// Local state
const renamingItemId = ref<string | null>(null)
const newItemTitle = ref('')
const showMenuForItem = ref<string | null>(null)

// Computed properties
const sortedItems = computed(() => {
  return [...props.items].sort((a, b) => {
    const timeA = new Date(a.timestamp).getTime()
    const timeB = new Date(b.timestamp).getTime()
    return timeB - timeA
  })
})

const themeClasses = computed(() => {
  return {
    purple: {
      primary: 'text-purple-400',
      button: 'bg-purple-500/80 hover:bg-purple-500',
      active: 'border-purple-500/50 bg-purple-500/10',
      hover: 'hover:bg-white/10'
    },
    blue: {
      primary: 'text-blue-400',
      button: 'bg-blue-600/20 hover:bg-blue-600/30 border border-blue-500/30',
      active: 'border-blue-500/30 bg-blue-600/20',
      hover: 'hover:bg-white/5'
    }
  }[props.theme]
})

// Helper functions
const formatTimestamp = (timestamp: Date | string | number): string => {
  const date = new Date(timestamp)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  
  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  
  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours}h ago`
  
  const diffDays = Math.floor(diffHours / 24)
  if (diffDays < 7) return `${diffDays}d ago`
  
  return date.toLocaleDateString()
}

// UI interaction handlers
const startRenaming = (item: SidebarItem) => {
  renamingItemId.value = item.id
  newItemTitle.value = item.title
  showMenuForItem.value = null
}

const finishRenaming = () => {
  if (renamingItemId.value && newItemTitle.value.trim()) {
    emit('rename-item', renamingItemId.value, newItemTitle.value.trim())
  }
  renamingItemId.value = null
  newItemTitle.value = ''
}

const cancelRenaming = () => {
  renamingItemId.value = null
  newItemTitle.value = ''
}

const toggleMenu = (itemId: string) => {
  showMenuForItem.value = showMenuForItem.value === itemId ? null : itemId
}

const handleSelectItem = (itemId: string) => {
  emit('select-item', itemId)
  showMenuForItem.value = null
}

const handleDeleteItem = (itemId: string, event: Event) => {
  event.stopPropagation()
  emit('delete-item', itemId)
  showMenuForItem.value = null
}

// Close menu when clicking outside
const closeMenu = () => {
  showMenuForItem.value = null
}
</script>

<template>
  <!-- Overlay mode wrapper -->
  <Transition v-if="displayMode === 'overlay'" name="overlay">
    <div v-if="$slots.default" class="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm" @click="emit('close')">
      <Transition name="slide">
        <div 
          :class="[width, 'fixed top-0 left-0 h-full z-50 bg-black/90 backdrop-blur-xl border-r border-white/20 shadow-2xl shadow-black/50 flex flex-col']" 
          @click.stop
        >
          <slot />
        </div>
      </Transition>
    </div>
  </Transition>

  <!-- Inline mode wrapper -->
  <div v-else-if="displayMode === 'inline'" :class="[width, 'border-r border-white/10 bg-white/5 backdrop-blur-sm flex flex-col']">
    <slot />
  </div>

  <!-- Default slot content (header, content, footer) -->
  <template v-if="!$slots.default">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-white/10">
      <div class="flex items-center gap-2">
        <component :is="icon" v-if="icon" :class="['w-4 h-4', themeClasses.primary]" />
        <span class="text-sm font-medium text-white/90">{{ title }}</span>
      </div>
      <button @click="emit('close')" class="rounded-full p-1 hover:bg-white/10 transition-colors text-white/70 hover:text-white">
        <XMarkIcon class="w-4 h-4" />
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-hidden flex flex-col">
      <!-- New Item Button -->
      <div v-if="showNewButton" class="p-3 border-b border-white/10">
        <button 
          @click="emit('new-item')" 
          :class="['w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg transition-colors text-sm font-medium text-white/90', themeClasses.button]"
        >
          <PlusIcon class="w-4 h-4" />
          <span>New {{ title.replace(/s$/, '') }}</span>
        </button>
      </div>

      <!-- Items List -->
      <div class="flex-1 overflow-y-auto px-4 pb-4" @click="closeMenu">
        <!-- Loading State -->
        <div v-if="isLoading" class="flex flex-col items-center gap-2 py-8">
          <div class="w-4 h-4 border-2 border-white/20 border-t-white/60 rounded-full animate-spin" />
          <span class="text-xs text-white/60">Loading...</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="items.length === 0" class="py-8 text-center">
          <component :is="icon" v-if="icon" class="w-8 h-8 text-white/20 mx-auto mb-2" />
          <p class="text-white/60 text-sm">{{ emptyMessage }}</p>
          <p v-if="emptySubMessage" class="text-white/40 text-xs mt-1">{{ emptySubMessage }}</p>
        </div>

        <!-- Items Grid -->
        <div v-else class="space-y-2 py-2">
          <div 
            v-for="item in sortedItems" 
            :key="item.id"
            class="rounded-lg transition-all duration-200 cursor-pointer p-3 border border-transparent"
            :class="[
              themeClasses.hover,
              item.id === currentItemId || item.isActive ? themeClasses.active : 'bg-white/5'
            ]"
            @click="handleSelectItem(item.id)"
          >
            <!-- Item Header -->
            <div class="flex items-center justify-between mb-2">
              <!-- Title (with rename functionality) -->
              <div v-if="renamingItemId === item.id" class="flex-1 mr-2">
                <input
                  v-model="newItemTitle"
                  @keyup.enter="finishRenaming"
                  @keyup.escape="cancelRenaming"
                  @blur="finishRenaming"
                  @click.stop
                  class="w-full px-2 py-1 text-sm bg-white/10 border border-white/20 rounded text-white/90 focus:outline-none focus:border-blue-500/50"
                  autofocus
                />
              </div>
              <span v-else class="text-xs font-medium text-white truncate flex-1 mr-2">
                {{ item.title }}
              </span>

              <!-- Actions Menu -->
              <div class="relative" @click.stop>
                <button 
                  @click="toggleMenu(item.id)" 
                  class="rounded-full p-1 hover:bg-white/10 transition-colors text-white/60 hover:text-white/90"
                >
                  <EllipsisVerticalIcon class="w-4 h-4" />
                </button>

                <!-- Dropdown Menu -->
                <Transition name="menu">
                  <div 
                    v-if="showMenuForItem === item.id" 
                    class="absolute right-0 top-8 bg-black/95 border border-white/20 rounded-lg shadow-xl z-50 py-1 min-w-32"
                  >
                    <button 
                      v-if="showRenameButton"
                      @click="startRenaming(item)" 
                      class="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-white/10 transition-colors text-white/80 hover:text-white/90"
                    >
                      <PencilIcon class="w-3 h-3" />
                      <span>Rename</span>
                    </button>
                    <button 
                      v-if="showDeleteButton"
                      @click="(e) => handleDeleteItem(item.id, e)" 
                      class="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-white/10 transition-colors text-red-400 hover:text-red-300 hover:bg-red-500/10"
                    >
                      <TrashIcon class="w-3 h-3" />
                      <span>Delete</span>
                    </button>
                  </div>
                </Transition>
              </div>
            </div>

            <!-- Item Metadata -->
            <div v-if="showTimestamps || showMetadata" class="space-y-1">
              <!-- Timestamp -->
              <div v-if="showTimestamps" class="flex items-center gap-1">
                <ClockIcon class="w-3 h-3 text-white/40" />
                <span class="text-xs text-white/40">{{ formatTimestamp(item.timestamp) }}</span>
                <span v-if="item.metadata?.duration" class="text-xs text-white/40">• {{ item.metadata.duration }}</span>
              </div>

              <!-- Additional Metadata -->
              <div v-if="showMetadata && item.subtitle" class="text-xs text-white/50">
                {{ item.subtitle }}
              </div>

              <!-- Preview -->
              <div v-if="showMetadata && item.metadata?.preview" class="text-xs text-white/60 line-clamp-2">
                {{ item.metadata.preview }}
              </div>

              <!-- Custom metadata slot -->
              <slot name="item-metadata" :item="item" />
            </div>
          </div>
        </div>
      </div>

      <!-- Footer (optional) -->
      <div v-if="showClearAllButton && items.length > 0" class="p-3 border-t border-white/10">
        <button 
          @click="emit('clear-all')" 
          class="w-full flex items-center justify-center gap-2 px-3 py-2 bg-white/5 hover:bg-white/10 text-white/60 hover:text-white/80 rounded-lg transition-colors text-sm font-medium border border-white/10"
        >
          <TrashIcon class="w-4 h-4" />
          <span>Clear All</span>
        </button>
      </div>
    </div>
  </template>
</template>

<style scoped>
/* Overlay transitions */
.overlay-enter-active,
.overlay-leave-active {
  transition: opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.overlay-enter-from,
.overlay-leave-to {
  opacity: 0;
}

/* Slide transitions */
.slide-enter-active,
.slide-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-enter-from,
.slide-leave-to {
  transform: translateX(-100%);
}

/* Menu transitions */
.menu-enter-active,
.menu-leave-active {
  transition: all 0.15s ease-out;
}

.menu-enter-from {
  opacity: 0;
  transform: translateY(-4px) scale(0.95);
}

.menu-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.95);
}

/* Scrollbar styling */
.overflow-y-auto::-webkit-scrollbar {
  width: 4px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}
</style>