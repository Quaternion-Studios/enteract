// storageService.ts - Handles chat persistence with enhanced reliability
import { invoke } from '@tauri-apps/api/core'
import type { ChatSession, SaveChatsPayload, LoadChatsResponse } from '../types/chat'

interface BackupInfo {
  filename: string
  backup_type: string
  size: number
  modified: number
}

export class StorageService {
  private static readonly STORAGE_KEY = 'user_chat_sessions.json'
  private static readonly MAX_RETRIES = 3
  private static readonly RETRY_DELAY = 1000 // 1 second
  private static saveQueue: ChatSession[] | null = null
  private static isSaving = false
  
  // Debounce utility
  private static debounce(func: Function, delay: number) {
    let timeoutId: number
    return (...args: any[]) => {
      clearTimeout(timeoutId)
      timeoutId = window.setTimeout(() => func.apply(null, args), delay)
    }
  }
  
  // Retry utility
  private static async retry<T>(
    fn: () => Promise<T>, 
    retries: number = StorageService.MAX_RETRIES,
    delay: number = StorageService.RETRY_DELAY
  ): Promise<T> {
    try {
      return await fn()
    } catch (error) {
      if (retries > 0) {
        console.warn(`Operation failed, retrying... (${retries} retries left)`, error)
        await new Promise(resolve => setTimeout(resolve, delay))
        return StorageService.retry(fn, retries - 1, delay * 2) // Exponential backoff
      }
      throw error
    }
  }

  static async saveAllChats(chatSessions: ChatSession[]) {
    // If already saving, queue the latest data
    if (StorageService.isSaving) {
      StorageService.saveQueue = chatSessions
      return
    }
    
    StorageService.isSaving = true
    
    try {
      const payload: SaveChatsPayload = { chats: chatSessions }
      
      // Use retry mechanism for robustness
      await StorageService.retry(async () => {
        await invoke('save_chat_sessions', { payload })
      })
      
      console.log('✅ Chat sessions saved successfully')
      
      // Check if there's queued data to save
      if (StorageService.saveQueue) {
        const queuedData = StorageService.saveQueue
        StorageService.saveQueue = null
        StorageService.isSaving = false
        // Recursively save queued data
        await StorageService.saveAllChats(queuedData)
      } else {
        StorageService.isSaving = false
      }
    } catch (error) {
      StorageService.isSaving = false
      console.error('❌ Failed to save chat sessions after retries:', error)
      
      // Attempt to save to localStorage as fallback
      try {
        localStorage.setItem('chat_sessions_backup', JSON.stringify(chatSessions))
        console.warn('⚠️ Saved to localStorage as fallback')
      } catch (localError) {
        console.error('❌ Failed to save to localStorage:', localError)
      }
      
      throw error // Re-throw for caller to handle
    }
  }

  // Debounced save function (1000ms delay)
  static debouncedSaveChats = StorageService.debounce(StorageService.saveAllChats, 1000)

  static async loadAllChats(): Promise<ChatSession[]> {
    try {
      // Use retry mechanism for robustness
      const response: LoadChatsResponse = await StorageService.retry(async () => {
        return await invoke('load_chat_sessions')
      })
      
      if (response.chats && response.chats.length > 0) {
        console.log(`✅ Loaded ${response.chats.length} chat sessions from backend`)
        return response.chats
      }
      
      // If no chats in backend, check localStorage backup
      const backup = localStorage.getItem('chat_sessions_backup')
      if (backup) {
        try {
          const backupChats = JSON.parse(backup)
          console.warn('⚠️ Loaded chat sessions from localStorage backup')
          // Try to save to backend
          StorageService.saveAllChats(backupChats).catch(console.error)
          return backupChats
        } catch (parseError) {
          console.error('Failed to parse localStorage backup:', parseError)
        }
      }
      
      return []
    } catch (error) {
      console.error('❌ Failed to load chat sessions:', error)
      
      // Attempt to load from localStorage as fallback
      const backup = localStorage.getItem('chat_sessions_backup')
      if (backup) {
        try {
          const backupChats = JSON.parse(backup)
          console.warn('⚠️ Using localStorage backup due to backend failure')
          return backupChats
        } catch (parseError) {
          console.error('Failed to parse localStorage backup:', parseError)
        }
      }
      
      return []
    }
  }
  
  // New backup management methods
  static async listBackups(): Promise<BackupInfo[]> {
    try {
      const backups = await invoke<BackupInfo[]>('list_backups')
      console.log(`✅ Found ${backups.length} backups`)
      return backups
    } catch (error) {
      console.error('❌ Failed to list backups:', error)
      return []
    }
  }
  
  static async restoreFromBackup(backupType: string, filename: string): Promise<void> {
    try {
      await invoke('restore_from_backup', { 
        backupType, 
        backupFilename: filename 
      })
      console.log('✅ Successfully restored from backup')
    } catch (error) {
      console.error('❌ Failed to restore from backup:', error)
      throw error
    }
  }
  
  // Export chats to JSON file
  static exportChatsToFile(chatSessions: ChatSession[]): void {
    try {
      const dataStr = JSON.stringify(chatSessions, null, 2)
      const dataBlob = new Blob([dataStr], { type: 'application/json' })
      const url = URL.createObjectURL(dataBlob)
      const link = document.createElement('a')
      link.href = url
      link.download = `enteract_chats_${new Date().toISOString().split('T')[0]}.json`
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      URL.revokeObjectURL(url)
      console.log('✅ Chats exported successfully')
    } catch (error) {
      console.error('❌ Failed to export chats:', error)
      throw error
    }
  }
  
  // Import chats from JSON file
  static async importChatsFromFile(file: File): Promise<ChatSession[]> {
    try {
      const text = await file.text()
      const chats = JSON.parse(text) as ChatSession[]
      
      // Validate the structure
      if (!Array.isArray(chats)) {
        throw new Error('Invalid file format: expected array of chat sessions')
      }
      
      // Basic validation of chat structure
      for (const chat of chats) {
        if (!chat.id || !chat.title || !Array.isArray(chat.history)) {
          throw new Error('Invalid chat session structure')
        }
      }
      
      console.log(`✅ Successfully imported ${chats.length} chat sessions`)
      return chats
    } catch (error) {
      console.error('❌ Failed to import chats:', error)
      throw error
    }
  }
}