<script setup lang="ts">
import { type PropType } from 'vue'
import { ArrowsPointingOutIcon, TrashIcon, ArrowDownTrayIcon } from '@heroicons/vue/24/outline'

interface OllamaStatus {
  status: string
  version?: string
}

interface OllamaModel {
  name: string
  size: number
  details?: { parameter_size?: string }
}

const props = defineProps({
  ollamaStatus: { type: Object as PropType<OllamaStatus>, required: true },
  isLoadingModels: { type: Boolean, required: true },
  modelsError: { type: String as PropType<string | null>, required: false, default: null },
  ollamaModels: { type: Array as PropType<OllamaModel[]>, required: true },
  selectedModel: { type: String as PropType<string | null>, required: false, default: null },
  pullingModel: { type: String as PropType<string | null>, required: false, default: null },
  deletingModel: { type: String as PropType<string | null>, required: false, default: null },
  getModelDisplayName: { type: Function as PropType<(model: any) => string>, required: true },
  formatModelSize: { type: Function as PropType<(size: number) => string>, required: true },
  fetchOllamaModels: { type: Function as PropType<(force?: boolean) => Promise<void> | void>, required: true },
  pullModel: { type: Function as PropType<(name: string) => Promise<void> | void>, required: true },
  deleteModel: { type: Function as PropType<(name: string) => Promise<void> | void>, required: true },
  clearModelsError: { type: Function as PropType<() => void>, required: true },
  onSelectModel: { type: Function as PropType<(name: string) => void>, required: true }
})
</script>

<template>
  <div class="settings-section">
    <div class="section-header">
      <h2 class="section-title">AI Models</h2>
      <p class="section-description">
        Manage AI models for transcription analysis and intelligent responses. Enteract runs models locally for privacy and performance.
      </p>
    </div>

    <div class="ollama-status">
      <div v-if="ollamaStatus.status === 'running'" class="status-good">
        <span class="text-green-400">● Model manager is running</span>
        <span v-if="ollamaStatus.version" class="text-white/60 text-xs ml-2">v{{ ollamaStatus.version }}</span>
      </div>
      <div v-else-if="ollamaStatus.status === 'not_running'" class="status-error">
        <span class="text-red-400">● Model manager is not running</span>
        <div class="text-white/60 text-xs mt-2 space-y-1">
          <p class="font-medium">To get started:</p>
          <ol class="list-decimal list-inside space-y-1 text-white/50">
            <li>Install Ollama from <a href="https://ollama.com" target="_blank" class="text-blue-400 hover:underline">ollama.com</a></li>
            <li>Open Terminal and run: <code class="bg-white/10 px-1 rounded">ollama serve</code></li>
            <li>Refresh this page to see available models</li>
          </ol>
          <p class="text-white/40 mt-2">For resource-constrained Macs, recommended models:</p>
          <p class="text-white/40">• gemma3:1b-it-qat (1GB) - Fastest</p>
          <p class="text-white/40">• phi3-mini (2GB) - Balanced</p>
          <p class="text-white/40">• qwen2.5:0.5b (500MB) - Smallest</p>
        </div>
      </div>
      <div v-else-if="ollamaStatus.status === 'checking'" class="status-loading">
        <span class="text-yellow-400">● Checking model manager status...</span>
      </div>
      <div v-else class="status-error">
        <span class="text-red-400">● Failed to connect to model manager</span>
      </div>
    </div>

    <div v-if="ollamaStatus.status === 'running'" class="models-section">
      <div class="models-header">
        <h3 class="text-white/90 font-medium">Available Models</h3>
        <button 
          @click="() => fetchOllamaModels(true)" 
          :disabled="isLoadingModels"
          class="refresh-btn"
          title="Refresh Models"
        >
          <ArrowsPointingOutIcon class="w-4 h-4" :class="{ 'animate-spin': isLoadingModels }" />
        </button>
      </div>

      <div v-if="modelsError" class="error-message">
        <span class="text-red-400 text-sm">{{ modelsError }}</span>
        <button @click="clearModelsError" class="ml-2 text-white/60 hover:text-white">×</button>
      </div>

      <div v-if="isLoadingModels" class="loading-state">
        <div class="animate-pulse text-white/60">Loading models...</div>
      </div>

      <div v-else-if="ollamaModels.length > 0" class="models-list">
        <div v-for="model in ollamaModels" :key="model.name" class="model-item">
          <div class="model-info">
            <div class="model-name">{{ getModelDisplayName(model) }}</div>
            <div class="model-details">
              <span class="model-size">{{ formatModelSize(model.size) }}</span>
              <span v-if="model.details?.parameter_size" class="model-params">
                {{ model.details.parameter_size }}
              </span>
            </div>
          </div>

          <div class="model-actions">
            <button
              @click="onSelectModel(model.name)"
              :class="{ 'active': selectedModel === model.name }"
              class="select-btn"
              title="Select Model"
            >
              {{ selectedModel === model.name ? '✓' : '○' }}
            </button>

            <button
              @click="deleteModel(model.name)"
              :disabled="deletingModel === model.name"
              class="delete-btn"
              title="Delete Model"
            >
              <TrashIcon v-if="deletingModel !== model.name" class="w-3 h-3" />
              <div v-else class="w-3 h-3 animate-spin">⟳</div>
            </button>
          </div>
        </div>
      </div>

      <div v-else class="no-models">
        <p class="text-white/60 text-sm">No models available</p>
        <p class="text-white/40 text-xs mt-1">Pull a model to get started</p>
      </div>

      <div class="pull-model-section">
        <h4 class="text-white/80 text-sm font-medium mb-2">Pull New Model</h4>
        <div class="popular-models">
          <button 
            v-for="modelName in ['gemma3:1b-it-qat', 'qwen2.5vl:3b', 'qwen2.5-coder:1.5b', 'deepseek-r1:1.5b', 'llama3.2']" 
            :key="modelName"
            @click="pullModel(modelName)"
            :disabled="pullingModel === modelName"
            class="model-pull-btn"
            :class="{ 
              'recommended': modelName === 'gemma3:1b-it-qat',
              'vision-model': modelName === 'qwen2.5vl:3b',
              'coding-model': modelName === 'qwen2.5-coder:1.5b',
              'research-model': modelName === 'deepseek-r1:1.5b'
            }"
          >
            <ArrowDownTrayIcon v-if="pullingModel !== modelName" class="w-3 h-3" />
            <div v-else class="w-3 h-3 animate-spin">⟳</div>
            <span>{{ modelName }}</span>
            <span v-if="modelName === 'gemma3:1b-it-qat'" class="recommended-badge">Enteract Agent</span>
            <span v-if="modelName === 'qwen2.5vl:3b'" class="vision-badge">Vision</span>
            <span v-if="modelName === 'qwen2.5-coder:1.5b'" class="coding-badge">Coding</span>
            <span v-if="modelName === 'deepseek-r1:1.5b'" class="research-badge">Research</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>


