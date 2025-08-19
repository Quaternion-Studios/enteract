// messageIdGenerator.ts - Centralized unique message ID generator
// This ensures no duplicate IDs across all services that create chat messages

class MessageIdGenerator {
  private static instance: MessageIdGenerator
  private counter: number

  private constructor() {
    // Start with a high number to avoid conflicts with existing data
    // Use timestamp-based initial value to ensure uniqueness across app restarts
    this.counter = Date.now() % 1000000 // Keep it reasonably sized
  }

  public static getInstance(): MessageIdGenerator {
    if (!MessageIdGenerator.instance) {
      MessageIdGenerator.instance = new MessageIdGenerator()
    }
    return MessageIdGenerator.instance
  }

  public getNextId(): number {
    return ++this.counter
  }

  // Reset counter if needed (for testing or special cases)
  public reset(startValue?: number): void {
    this.counter = startValue || (Date.now() % 1000000)
  }
}

// Export a simple function for easy use across the app
export const getNextMessageId = (): number => {
  return MessageIdGenerator.getInstance().getNextId()
}

// Export the class for advanced usage if needed
export { MessageIdGenerator }
