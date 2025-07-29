import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Types for GPU detection
export interface GpuInfo {
  name: string
  driver_version?: string
  memory_total?: number // in MB
  memory_used?: number // in MB
  temperature?: number // in Celsius
  utilization?: number // percentage
  vendor: 'Nvidia' | 'Amd' | 'Intel' | 'Unknown'
  pci_id?: string
}

export interface GpuStats {
  gpus: GpuInfo[]
  last_updated: number // timestamp
}

export const useGpuInfo = () => {
  const gpuStats = ref<GpuStats>({
    gpus: [],
    last_updated: 0
  })
  
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  
  // Computed properties
  const gpus = computed(() => gpuStats.value.gpus)
  const hasGpus = computed(() => gpus.value.length > 0)
  const lastUpdated = computed(() => new Date(gpuStats.value.last_updated * 1000))
  
  // Get primary GPU (first one)
  const primaryGpu = computed(() => gpus.value[0] || null)
  
  // Get GPUs by vendor
  const nvidiaGpus = computed(() => gpus.value.filter(gpu => gpu.vendor === 'Nvidia'))
  const amdGpus = computed(() => gpus.value.filter(gpu => gpu.vendor === 'Amd'))
  const intelGpus = computed(() => gpus.value.filter(gpu => gpu.vendor === 'Intel'))
  
  // Check if caching is still valid (5 minutes)
  const isCacheValid = () => {
    const now = Date.now() / 1000
    const age = now - gpuStats.value.last_updated
    return age < 300 // 5 minutes
  }
  
  const fetchGpuInfo = async (forceRefresh = false) => {
    // Return cached data if still valid and not forcing refresh
    if (!forceRefresh && isCacheValid() && hasGpus.value) {
      console.log('📋 Using cached GPU info')
      return gpuStats.value
    }
    
    isLoading.value = true
    error.value = null
    
    try {
      console.log('🖥️ Fetching GPU information...')
      const stats = await invoke<GpuStats>('get_gpu_info')
      gpuStats.value = stats
      console.log('✅ GPU info loaded:', stats)
      return stats
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error'
      console.error('❌ Failed to fetch GPU info:', errorMessage)
      error.value = `Failed to detect GPUs: ${errorMessage}`
      throw err
    } finally {
      isLoading.value = false
    }
  }
  
  const fetchGpuUtilization = async () => {
    try {
      console.log('📊 Fetching GPU utilization...')
      const utilization = await invoke<GpuInfo[]>('get_gpu_utilization')
      
      // Update utilization data in existing GPUs
      utilization.forEach(updatedGpu => {
        const existingGpuIndex = gpuStats.value.gpus.findIndex(gpu => gpu.name === updatedGpu.name)
        if (existingGpuIndex !== -1) {
          // Update utilization, temperature, and memory usage
          gpuStats.value.gpus[existingGpuIndex] = {
            ...gpuStats.value.gpus[existingGpuIndex],
            utilization: updatedGpu.utilization,
            temperature: updatedGpu.temperature,
            memory_used: updatedGpu.memory_used
          }
        }
      })
      
      // Update timestamp
      gpuStats.value.last_updated = Math.floor(Date.now() / 1000)
      
      return utilization
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error'
      console.error('❌ Failed to fetch GPU utilization:', errorMessage)
      throw err
    }
  }
  
  const clearError = () => {
    error.value = null
  }
  
  const refresh = async () => {
    await fetchGpuInfo(true)
  }
  
  // Format memory size for display
  const formatMemorySize = (sizeInMB?: number) => {
    if (!sizeInMB) return 'Unknown'
    
    if (sizeInMB >= 1024) {
      return `${(sizeInMB / 1024).toFixed(1)} GB`
    }
    return `${sizeInMB} MB`
  }
  
  // Get vendor color class for styling
  const getVendorColorClass = (vendor: GpuInfo['vendor']) => {
    switch (vendor) {
      case 'Nvidia':
        return 'vendor-nvidia'
      case 'Amd':
        return 'vendor-amd'
      case 'Intel':
        return 'vendor-intel'
      default:
        return 'vendor-unknown'
    }
  }
  
  // Check if GPU supports real-time monitoring
  const supportsRealTimeMonitoring = (gpu: GpuInfo) => {
    return gpu.vendor === 'Nvidia' && gpu.utilization !== undefined
  }
  
  // Get GPU temperature status
  const getTemperatureStatus = (temperature?: number) => {
    if (!temperature) return { status: 'unknown', color: 'text-gray-400' }
    
    if (temperature < 60) {
      return { status: 'cool', color: 'text-green-400' }
    } else if (temperature < 80) {
      return { status: 'warm', color: 'text-yellow-400' }
    } else {
      return { status: 'hot', color: 'text-red-400' }
    }
  }
  
  // Get memory usage percentage
  const getMemoryUsagePercentage = (gpu: GpuInfo) => {
    if (!gpu.memory_used || !gpu.memory_total) return 0
    return Math.round((gpu.memory_used / gpu.memory_total) * 100)
  }
  
  // Format PCI ID for display (truncate long Windows PCI paths)
  const formatPciId = (pciId?: string) => {
    if (!pciId) return 'Unknown'
    
    // For Windows-style PCI paths, extract the meaningful part
    if (pciId.includes('PCI\\VEN_')) {
      const match = pciId.match(/VEN_([A-F0-9]{4})&DEV_([A-F0-9]{4})/)
      if (match) {
        return `${match[1]}:${match[2]}`
      }
    }
    
    // For Linux-style (already short), keep as is
    if (pciId.match(/^[0-9a-f]+:[0-9a-f]+\.[0-9]$/i)) {
      return pciId
    }
    
    // If still very long, truncate but keep meaningful parts
    if (pciId.length > 20) {
      return pciId.substring(0, 16) + '...'
    }
    
    return pciId
  }
  
  return {
    // State
    gpuStats,
    isLoading,
    error,
    
    // Computed
    gpus,
    hasGpus,
    lastUpdated,
    primaryGpu,
    nvidiaGpus,
    amdGpus,
    intelGpus,
    
    // Methods
    fetchGpuInfo,
    fetchGpuUtilization,
    clearError,
    refresh,
    isCacheValid,
    
    // Utilities
    formatMemorySize,
    getVendorColorClass,
    supportsRealTimeMonitoring,
    getTemperatureStatus,
    getMemoryUsagePercentage,
    formatPciId
  }
}