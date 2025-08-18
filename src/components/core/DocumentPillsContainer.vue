<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { DocumentTextIcon, ChevronLeftIcon, ChevronRightIcon, XMarkIcon } from '@heroicons/vue/24/outline'
import { truncateText } from '@/utils/formatters'
import type { EnhancedDocument } from '@/services/enhancedRagService'

interface Props {
  documents: EnhancedDocument[]
  selectedDocumentIds: Set<string>
  maxVisible?: number
  limitInfo?: { current: number; max: number; isAtLimit: boolean }
}

interface Emits {
  (e: 'deselect', documentId: string): void
  (e: 'showAll'): void
}

const props = withDefaults(defineProps<Props>(), {
  maxVisible: 3
})

const emit = defineEmits<Emits>()

// State
const scrollContainer = ref<HTMLElement>()
const showScrollLeft = ref(false)
const showScrollRight = ref(false)
const isExpanded = ref(false)

// Computed
const selectedDocuments = computed(() => {
  return props.documents.filter(doc => props.selectedDocumentIds.has(doc.id))
})

const visibleDocuments = computed(() => {
  if (isExpanded.value) {
    return selectedDocuments.value
  }
  return selectedDocuments.value.slice(0, props.maxVisible)
})

const hiddenCount = computed(() => {
  return Math.max(0, selectedDocuments.value.length - props.maxVisible)
})

// Methods
const handleScroll = () => {
  if (!scrollContainer.value) return
  
  const { scrollLeft, scrollWidth, clientWidth } = scrollContainer.value
  showScrollLeft.value = scrollLeft > 0
  showScrollRight.value = scrollLeft < scrollWidth - clientWidth - 1
}

const scrollLeft = () => {
  if (!scrollContainer.value) return
  scrollContainer.value.scrollBy({ left: -200, behavior: 'smooth' })
}

const scrollRight = () => {
  if (!scrollContainer.value) return
  scrollContainer.value.scrollBy({ left: 200, behavior: 'smooth' })
}

const toggleExpanded = () => {
  isExpanded.value = !isExpanded.value
}

const handleDeselect = (documentId: string) => {
  emit('deselect', documentId)
}

// Watch for changes to update scroll indicators
watch(selectedDocuments, () => {
  setTimeout(handleScroll, 100)
})
</script>

<template>
  <div v-if="selectedDocuments.length > 0" class="document-pills-container">
    <!-- Scroll Left Button -->
    <button
      v-if="showScrollLeft"
      @click="scrollLeft"
      class="scroll-button scroll-left"
      aria-label="Scroll left"
    >
      <ChevronLeftIcon class="w-3 h-3" />
    </button>

    <!-- Pills Container -->
    <div
      ref="scrollContainer"
      class="pills-scroll-container"
      :class="{ expanded: isExpanded }"
      @scroll="handleScroll"
    >
      <TransitionGroup name="pill-list" tag="div" class="pills-wrapper">
        <div
          v-for="doc in visibleDocuments"
          :key="doc.id"
          class="document-pill"
          :title="`${doc.file_name} (${doc.file_size} bytes)`"
        >
          <DocumentTextIcon class="pill-icon" />
          <span class="pill-text">{{ truncateText(doc.file_name, 20) }}</span>
          <button 
            class="pill-close" 
            @click.stop="handleDeselect(doc.id)"
            aria-label="Remove document"
          >
            <XMarkIcon class="w-3 h-3" />
          </button>
        </div>
      </TransitionGroup>

      <!-- More Indicator -->
      <button
        v-if="hiddenCount > 0 && !isExpanded"
        @click="toggleExpanded"
        class="more-pill"
        :title="`Show ${hiddenCount} more document${hiddenCount > 1 ? 's' : ''}`"
      >
        +{{ hiddenCount }}
      </button>
    </div>

    <!-- Scroll Right Button -->
    <button
      v-if="showScrollRight"
      @click="scrollRight"
      class="scroll-button scroll-right"
      aria-label="Scroll right"
    >
      <ChevronRightIcon class="w-3 h-3" />
    </button>

    <!-- Limit Indicator -->
    <div 
      v-if="limitInfo" 
      class="limit-indicator"
      :class="{ 'at-limit': limitInfo.isAtLimit }"
      :title="`${limitInfo.current} of ${limitInfo.max} documents selected`"
    >
      {{ limitInfo.current }}/{{ limitInfo.max }}
    </div>
  </div>
</template>

<style scoped>
.document-pills-container {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  position: relative;
  max-width: 100%;
  padding: 0.5rem 0;
}

.pills-scroll-container {
  display: flex;
  gap: 0.5rem;
  overflow-x: auto;
  scroll-behavior: smooth;
  scrollbar-width: none;
  -ms-overflow-style: none;
  flex: 1;
  min-width: 0;
}

.pills-scroll-container::-webkit-scrollbar {
  display: none;
}

.pills-scroll-container.expanded {
  flex-wrap: wrap;
  overflow-x: visible;
}

.pills-wrapper {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.document-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.5rem;
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 9999px;
  color: rgba(255, 255, 255, 0.9);
  font-size: 0.75rem;
  white-space: nowrap;
  flex-shrink: 0;
  transition: all 0.2s;
}

.document-pill:hover {
  background: rgba(59, 130, 246, 0.2);
  border-color: rgba(59, 130, 246, 0.5);
}

.pill-icon {
  width: 0.875rem;
  height: 0.875rem;
  color: rgba(147, 197, 253, 0.8);
}

.pill-text {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pill-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1rem;
  height: 1rem;
  margin-left: 0.125rem;
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.6);
  cursor: pointer;
  transition: all 0.2s;
  padding: 0;
}

.pill-close:hover {
  color: rgba(255, 255, 255, 0.9);
  background: rgba(239, 68, 68, 0.2);
  border-radius: 50%;
}

.more-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.25rem 0.625rem;
  background: rgba(107, 114, 128, 0.1);
  border: 1px solid rgba(107, 114, 128, 0.3);
  border-radius: 9999px;
  color: rgba(255, 255, 255, 0.7);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
}

.more-pill:hover {
  background: rgba(107, 114, 128, 0.2);
  border-color: rgba(107, 114, 128, 0.5);
  color: rgba(255, 255, 255, 0.9);
}

.scroll-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  background: rgba(30, 41, 59, 0.8);
  border: 1px solid rgba(71, 85, 105, 0.5);
  border-radius: 0.375rem;
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
}

.scroll-button:hover {
  background: rgba(51, 65, 85, 0.8);
  color: rgba(255, 255, 255, 0.9);
}

.limit-indicator {
  display: flex;
  align-items: center;
  padding: 0.25rem 0.5rem;
  background: rgba(34, 197, 94, 0.1);
  border: 1px solid rgba(34, 197, 94, 0.3);
  border-radius: 0.375rem;
  color: rgba(134, 239, 172, 0.9);
  font-size: 0.6875rem;
  font-weight: 500;
  transition: all 0.2s;
}

.limit-indicator.at-limit {
  background: rgba(239, 68, 68, 0.1);
  border-color: rgba(239, 68, 68, 0.3);
  color: rgba(252, 165, 165, 0.9);
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

/* Transitions */
.pill-list-enter-active,
.pill-list-leave-active {
  transition: all 0.3s ease;
}

.pill-list-enter-from {
  opacity: 0;
  transform: scale(0.8);
}

.pill-list-leave-to {
  opacity: 0;
  transform: scale(0.8);
}

.pill-list-move {
  transition: transform 0.3s ease;
}

/* Responsive */
@media (max-width: 640px) {
  .document-pill {
    font-size: 0.6875rem;
    padding: 0.1875rem 0.375rem;
  }
  
  .pill-text {
    max-width: 100px;
  }
  
  .pill-icon {
    width: 0.75rem;
    height: 0.75rem;
  }
}
</style>