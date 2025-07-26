import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useOllamaCache } from '../stores/ollamaCache'

// Types for Ollama
interface OllamaModel {
  name: string
  modified_at: string
  size: number
  digest: string
  details?: {
    format: string
    family: string
    parameter_size: string
    quantization_level: string
  }
}

interface OllamaStatus {
  status: string
  version?: string
}

export const useAIModels = () => {
  const cache = useOllamaCache()
  const cachedData = cache.getCache()
  
  const ollamaModels = computed(() => cachedData.models)
  const ollamaStatus = computed(() => cachedData.status)
  const selectedModel = computed({
    get: () => cachedData.selectedModel,
    set: (value) => cache.setSelectedModel(value)
  })
  
  const isLoadingModels = ref(false)
  const modelsError = ref<string | null>(null)
  const pullingModel = ref<string | null>(null)
  const deletingModel = ref<string | null>(null)

  const fetchOllamaStatus = async (forceRefresh = false) => {
    // Return cached status if still valid and not forcing refresh
    if (!forceRefresh && cache.isCacheValid() && cachedData.status.status !== 'checking') {
      console.log('📋 Using cached Ollama status')
      return cachedData.status
    }
    
    try {
      const status = await invoke<OllamaStatus>('get_ollama_status')
      cache.setStatus(status)
      return status
    } catch (error) {
      console.error('Failed to get Ollama status:', error)
      cache.setStatus({ status: 'error' })
      return { status: 'error' }
    }
  }

  const fetchOllamaModels = async (forceRefresh = false) => {
    // Return cached models if still valid and not forcing refresh
    if (!forceRefresh && cache.isCacheValid() && cachedData.models.length > 0) {
      console.log('📋 Using cached Ollama models')
      return
    }
    
    isLoadingModels.value = true
    modelsError.value = null
    
    try {
      const models = await invoke<OllamaModel[]>('get_ollama_models')
      cache.setModels(models)
      console.log('📋 Fetched Ollama models:', models)
      
      // Auto-select gemma3:1b-it-qat if available and no model is selected
      if (!selectedModel.value) {
        const gemmaModel = models.find(model => 
          model.name.includes('gemma3:1b-it-qat') || 
          model.name.includes('gemma3') ||
          model.name.includes('gemma')
        )
        
        if (gemmaModel) {
          selectedModel.value = gemmaModel.name
          console.log('🎯 Auto-selected Gemma model:', gemmaModel.name)
        } else if (models.length > 0) {
          // Fallback to first available model
          selectedModel.value = models[0].name
          console.log('🎯 Auto-selected first available model:', models[0].name)
        }
      }
      
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      modelsError.value = message
      console.error('Failed to fetch Ollama models:', error)
    } finally {
      isLoadingModels.value = false
    }
  }

  const pullModel = async (modelName: string) => {
    pullingModel.value = modelName
    
    try {
      const result = await invoke<string>('pull_ollama_model', { modelName })
      console.log('📥 Pull result:', result)
      // Refresh models list after pulling (force refresh)
      setTimeout(() => {
        cache.clearCache()
        fetchOllamaModels(true)
      }, 2000)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      console.error('Failed to pull model:', error)
      modelsError.value = `Failed to pull ${modelName}: ${message}`
    } finally {
      pullingModel.value = null
    }
  }

  const deleteModel = async (modelName: string) => {
    if (!confirm(`Are you sure you want to delete the model "${modelName}"?`)) {
      return
    }
    
    deletingModel.value = modelName
    
    try {
      const result = await invoke<string>('delete_ollama_model', { modelName })
      console.log('🗑️ Delete result:', result)
      // Refresh models list after deletion (force refresh)
      cache.clearCache()
      await fetchOllamaModels(true)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      console.error('Failed to delete model:', error)
      modelsError.value = `Failed to delete ${modelName}: ${message}`
    } finally {
      deletingModel.value = null
    }
  }

  const formatModelSize = (size: number): string => {
    const gb = size / (1024 * 1024 * 1024)
    return `${gb.toFixed(1)} GB`
  }

  const getModelDisplayName = (model: OllamaModel): string => {
    return model.name.split(':')[0] || model.name
  }

  return {
    ollamaModels,
    ollamaStatus,
    isLoadingModels,
    modelsError,
    selectedModel,
    pullingModel,
    deletingModel,
    fetchOllamaStatus,
    fetchOllamaModels,
    pullModel,
    deleteModel,
    formatModelSize,
    getModelDisplayName
  }
} 