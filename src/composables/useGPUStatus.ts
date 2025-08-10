import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface GPUInfo {
  name: string
  vendor: string
  memory_mb?: number
  driver_version?: string
}

export interface GPUAccelerationStatus {
  enabled: boolean
  layers: number
  gpus: GPUInfo[]
}

export function useGPUStatus() {
  const gpuStatus = ref<GPUAccelerationStatus | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const checkGPUStatus = async () => {
    try {
      isLoading.value = true
      error.value = null
      
      const status = await invoke<GPUAccelerationStatus>('get_gpu_acceleration_status')
      gpuStatus.value = status
      
      console.log('🎮 GPU Acceleration Status:', status)
      
      if (status.enabled) {
        console.log(`✅ GPU acceleration enabled with ${status.layers} layers`)
        status.gpus.forEach(gpu => {
          console.log(`  - ${gpu.vendor} ${gpu.name}: ${gpu.memory_mb}MB VRAM`)
        })
      } else {
        console.log('⚠️ GPU acceleration disabled, using CPU')
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to get GPU status'
      console.error('Failed to get GPU status:', err)
    } finally {
      isLoading.value = false
    }
  }

  const getStatusMessage = () => {
    if (!gpuStatus.value) return 'Checking GPU...'
    
    if (gpuStatus.value.enabled) {
      const gpu = gpuStatus.value.gpus[0]
      if (gpu) {
        return `GPU Accelerated: ${gpu.vendor} ${gpu.name} (${gpuStatus.value.layers} layers)`
      }
      return `GPU Accelerated (${gpuStatus.value.layers} layers)`
    }
    
    return 'CPU Mode (No GPU acceleration)'
  }

  const getStatusColor = () => {
    if (!gpuStatus.value) return 'text-yellow-500'
    return gpuStatus.value.enabled ? 'text-green-500' : 'text-orange-500'
  }

  onMounted(() => {
    checkGPUStatus()
  })

  return {
    gpuStatus,
    isLoading,
    error,
    checkGPUStatus,
    getStatusMessage,
    getStatusColor
  }
}