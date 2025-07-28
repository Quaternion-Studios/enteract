<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useWindowManager } from './composables/useWindowManager'
import { useAIModels } from './composables/useAIModels'
import ControlPanel from './components/core/ControlPanel.vue'
import ChatSidebarAdapter from './components/core/ChatSidebarAdapter.vue'

// Initialize window manager
const { initializeWindow } = useWindowManager()

// AI Models management for chat sidebar
const { selectedModel } = useAIModels()

// Chat drawer state
const isChatDrawerOpen = ref(false)

// Reference to the ControlPanel component
const controlPanelRef = ref<InstanceType<typeof ControlPanel>>()

const toggleChatDrawer = () => {
  isChatDrawerOpen.value = !isChatDrawerOpen.value
  console.log(`💬 Chat drawer ${isChatDrawerOpen.value ? 'opened' : 'closed'}`)
}

const closeChatDrawer = () => {
  isChatDrawerOpen.value = false
  console.log('💬 Chat drawer closed')
}

const handleOpenChatWindow = () => {
  // Access the ControlPanel's openChatWindow method
  if (controlPanelRef.value && controlPanelRef.value.openChatWindow) {
    controlPanelRef.value.openChatWindow()
  }
}


onMounted(() => {
  initializeWindow()
  
  // Listen for chat drawer toggle events from ChatWindow
  window.addEventListener('toggle-chat-drawer', toggleChatDrawer)
})
</script>

<template>
  <div class="app-root">
    <ControlPanel 
      ref="controlPanelRef"
      @toggle-chat-drawer="toggleChatDrawer" 
    />
    
    <!-- Chat Drawer -->
    <ChatSidebarAdapter 
      :is-open="isChatDrawerOpen"
      :selected-model="selectedModel"
      @close="closeChatDrawer"
      @open-chat-window="handleOpenChatWindow"
    />
  </div>
</template>

<style>
/* Reset everything to ensure no extra space */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body {
  margin: 0;
  padding: 0;
  height: auto;
  width: auto;
  overflow: hidden;
  background: transparent;
}

#app {
  margin: 0;
  padding: 0;
  height: auto;
  width: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
}

.app-root {
  margin: 0;
  padding: 0;
  height: auto;
  width: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
}

/* Remove any default body styling */
body {
  background: transparent !important;
  overflow: hidden !important;
}

/* Ensure no scrollbars */
::-webkit-scrollbar {
  display: none;
}

html {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>


