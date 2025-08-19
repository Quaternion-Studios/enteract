// fileService.ts - Handles file upload functionality
import { SessionManager } from './sessionManager'
import { getNextMessageId } from './messageIdGenerator'

export class FileService {
  private static scrollChatToBottom: () => void
  private static ragDocumentsComposable: any = null

  static init(scrollCallback: () => void) {
    FileService.scrollChatToBottom = scrollCallback
  }

  static setRagComposable(ragComposable: any) {
    FileService.ragDocumentsComposable = ragComposable
  }

  static triggerFileUpload(fileInput: HTMLInputElement | undefined) {
    fileInput?.click()
  }

  static async handleFileUpload(event: Event, showChatWindow: any) {
    const input = event.target as HTMLInputElement
    const files = input.files
    if (files) {
      // Auto-open chat window if not already open
      if (!showChatWindow.value) {
        showChatWindow.value = true
        console.log('💬 Chat window auto-opened for file upload')
      }

      for (let i = 0; i < files.length; i++) {
        const file = files[i]
        try {
          // Enhanced file upload indication
          console.log('📁 File selected:', file.name, file.type, file.size)
          
          // Add file upload message to current chat
          SessionManager.addMessageToCurrentChat({
            id: getNextMessageId(),
            sender: 'system',
            text: `📁 Uploading: **${file.name}** (${file.type}, ${(file.size / 1024).toFixed(1)} KB)...`,
            timestamp: new Date(),
            messageType: 'text'
          })
          
          // Upload to RAG system if available
          if (FileService.ragDocumentsComposable) {
            const document = await FileService.ragDocumentsComposable.uploadDocument(file)
            if (document) {
              
              // Update message with success
              SessionManager.addMessageToCurrentChat({
                id: getNextMessageId(),
                sender: 'system',
                text: `✅ Document **${file.name}** uploaded and indexed successfully!\n\n` +
                      `📊 Document ID: ${document.id}\n` +
                      `📝 Extracted ${document.content.length} characters\n` +
                      `🔍 Ready for RAG-powered questions\n\n` +
                      `Use @${file.name} to reference this document in your questions.`,
                timestamp: new Date(),
                messageType: 'text',
                metadata: {
                  documentId: document.id,
                  fileName: file.name,
                  fileType: file.type
                }
              })
            }
          } else {
            // Fallback to simple upload notification
            SessionManager.addMessageToCurrentChat({
              id: getNextMessageId(),
              sender: 'system',
              text: `✅ File ready for analysis. You can now ask questions about this ${file.type.includes('image') ? 'image' : 'document'}.`,
              timestamp: new Date(),
              messageType: 'text'
            })
          }
          
          // Auto-scroll to show the uploaded file message
          setTimeout(() => {
            FileService.scrollChatToBottom()
          }, 100)
          
        } catch (error) {
          console.error('File upload error:', error)
          SessionManager.addMessageToCurrentChat({
            id: getNextMessageId(),
            sender: 'system',
            text: `❌ File upload failed: ${error}`,
            timestamp: new Date(),
            messageType: 'text'
          })
        }
      }
      
      // Auto-scroll to show files
      setTimeout(() => {
        FileService.scrollChatToBottom()
      }, 150)
    }
    // Reset input
    input.value = ''
  }
}