// types.ts - Shared type definitions
export interface ChatMessage {
    id: number
    sender: 'user' | 'assistant' | 'system' | 'transcription'
    text: string
    timestamp: Date
    messageType: 'text'
    isInterim?: boolean
    confidence?: number
    isStreaming?: boolean
    sessionId?: string
  }
  
  export interface ChatSession {
    id: string
    title: string
    history: ChatMessage[]
    createdAt: string
    updatedAt: string
    modelId?: string
  }
  
  export interface SaveChatsPayload {
    chats: ChatSession[]
  }
  
  export interface LoadChatsResponse {
    chats: ChatSession[]
  }
  
  export interface ScreenshotResponse {
    image_base64: string
    width: number
    height: number
  }
  
  export interface StreamEvent {
    type: 'start' | 'chunk' | 'error' | 'complete'
    text?: string
    model?: string
    error?: string
    done?: boolean
  }