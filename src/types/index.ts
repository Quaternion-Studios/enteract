export interface ChatMessage {
  id: number
  text: string
  sender: 'user' | 'assistant' | 'transcription' | 'system'
  timestamp: Date
  isInterim?: boolean
  confidence?: number
  source?: 'web-speech' | 'whisper' | 'typed' | 'clipboard' | 'file-upload' | 'drag-drop'
  // Enhanced content support
  attachments?: MessageAttachment[]
  thinking?: ThinkingProcess
  messageType?: 'text' | 'image' | 'document' | 'mixed' | 'thinking'
  metadata?: MessageMetadata
}

// Chat Session Management Interfaces
export interface ChatSession {
  id: string
  title: string
  history: ChatMessage[]
  createdAt: string
  updatedAt: string
  modelId?: string
}

// Chat file data structures for Rust backend
export interface ChatMessageFile {
  id: string
  type: 'image' | 'document' | 'audio' | 'video'
  name: string
  size: number
  mimeType: string
  url?: string
  base64Data?: string
  thumbnail?: string
  extractedText?: string
  dimensions?: FileDimensions
  uploadProgress?: number
  uploadStatus?: 'uploading' | 'completed' | 'failed' | 'processing'
  error?: string
}

export interface FileDimensions {
  width: number
  height: number
}

// Backend payload interfaces
export interface SaveChatsPayload {
  chats: ChatSession[]
}

export interface LoadChatsResponse {
  chats: ChatSession[]
}

export interface MessageAttachment {
  id: string
  type: 'image' | 'document' | 'audio' | 'video'
  name: string
  size: number
  mimeType: string
  url?: string // For display
  base64Data?: string // For processing
  thumbnail?: string // Base64 thumbnail for images
  extractedText?: string // For documents
  dimensions?: { width: number; height: number } // For images/videos
  uploadProgress?: number // 0-100
  uploadStatus?: 'uploading' | 'completed' | 'failed' | 'processing'
  error?: string
}

export interface ThinkingProcess {
  isVisible: boolean
  content: string
  isStreaming: boolean
  steps?: ThinkingStep[]
}

export interface ThinkingStep {
  id: string
  title: string
  content: string
  timestamp: Date
  status: 'thinking' | 'completed' | 'current'
}

export interface MessageMetadata {
  agentType?: 'enteract' | 'vision' | 'deep_research'
  model?: string
  tokens?: number
  processingTime?: number
  analysisType?: string[]
  searchQueries?: string[]
  sources?: string[]
}

// File upload types
export interface FileUploadConfig {
  maxFileSize: number // in bytes
  allowedImageTypes: string[]
  allowedDocumentTypes: string[]
  maxFiles: number
}

export interface UploadProgress {
  fileId: string
  fileName: string
  progress: number
  status: 'uploading' | 'processing' | 'completed' | 'failed'
  error?: string
}

export interface AppState {
  micEnabled: boolean
  windowCollapsed: boolean
  isRecording: boolean
}

export interface WindowPosition {
  x: number
  y: number
}

export interface TransparencyState {
  level: number
  isTransparent: boolean
  isClickThrough: boolean
  isVisible: boolean
}

export interface TransparencyPresets {
  invisible: () => Promise<void>
  ghostMode: () => Promise<void>
  semiTransparent: () => Promise<void>
  solid: () => Promise<void>
}

// Re-export speech transcription types
export * from './speechTranscription'

// Re-export audio types
export * from './audio' 