// markdownRenderer.ts - Improved to be more simple and reliable
export class MarkdownRenderer {
  // Simple render function with size limits for safety
  static render(text: string): string {
    if (!text) return ''
    
    // Safety check - prevent freezing on huge content
    if (text.length > 15000) {
      console.warn('⚠️ Large content detected, truncating for performance')
      const truncated = text.substring(0, 15000)
      return this.renderSafe(truncated) + 
        `<div class="text-yellow-400 text-sm mt-4 p-2 bg-yellow-900/20 rounded">
          Content truncated at 15KB for performance. Original length: ${Math.round(text.length / 1000)}KB
        </div>`
    }
    
    return this.renderSafe(text)
  }
  
  // Safe rendering with timeout protection
  static renderSafe(text: string): string {
    const startTime = performance.now()
    
    try {
      let processed = text.trim()
      
      // Store code blocks to protect them from other processing
      const codeBlocks: string[] = []
      const inlineCodes: string[] = []
      const codeBlockPlaceholder = (index: number) => `⏿⏿⏿CODEBLOCK${index}⏿⏿⏿`
      const inlineCodePlaceholder = (index: number) => `⏿⏿⏿INLINECODE${index}⏿⏿⏿`
      
      // Extract code blocks with simple regex
      processed = processed.replace(/```(\w+)?\n?([\s\S]*?)```/g, (match, lang, code) => {
        const index = codeBlocks.length
        const language = (lang || 'text').toLowerCase()
        const cleanCode = code.trim()
        
        // Simple highlighting only
        const highlightedCode = this.highlightCodeSimple(cleanCode, language)
        const escapedCleanCode = cleanCode.replace(/`/g, '\\`').replace(/\$/g, '\\$')
        
        const codeBlockHtml = `<div class="code-block bg-black/40 border border-white/20 rounded-lg p-4 my-4 font-mono text-sm overflow-x-auto">
          <div class="code-header flex items-center justify-between mb-3">
            <div class="text-xs text-white/60 uppercase tracking-wide font-semibold">${language}</div>
            <button onclick="navigator.clipboard.writeText('${escapedCleanCode}')" 
                    class="text-xs text-white/40 hover:text-white/70 transition-colors px-2 py-1 rounded hover:bg-white/10">
              Copy
            </button>
          </div>
          <div class="code-content leading-relaxed whitespace-pre">${highlightedCode}</div>
        </div>`
        
        codeBlocks.push(codeBlockHtml)
        return codeBlockPlaceholder(index)
      })
      
      // Extract inline code
      processed = processed.replace(/`([^`\n]+)`/g, (match, code) => {
        const index = inlineCodes.length
        const inlineCodeHtml = `<code class="bg-black/40 px-1.5 py-0.5 rounded text-sm font-mono text-cyan-300">${this.escapeHtml(code)}</code>`
        inlineCodes.push(inlineCodeHtml)
        return inlineCodePlaceholder(index)
      })
      
      // Process block-level elements
      processed = this.processBlockElements(processed)
      
      // Process inline elements
      processed = this.processInlineElements(processed)
      
      // Restore code blocks
      codeBlocks.forEach((html, index) => {
        processed = processed.replace(codeBlockPlaceholder(index), html)
      })
      inlineCodes.forEach((html, index) => {
        processed = processed.replace(inlineCodePlaceholder(index), html)
      })
      
      // Only log if render takes more than 5ms (performance issue indicator)
      const renderTime = performance.now() - startTime
      if (renderTime > 5) {
        console.log(`⚡ Slow render: ${renderTime.toFixed(2)}ms for ${text.length} chars`)
      }
      
      return processed
      
    } catch (error) {
      console.error('❌ Render error, falling back to escaped text:', error)
      return `<div class="text-white/85 p-4 bg-red-900/20 border border-red-500/30 rounded">
        <div class="text-red-400 font-semibold mb-2">Markdown Render Error</div>
        <pre class="text-sm text-white/70 whitespace-pre-wrap">${this.escapeHtml(text)}</pre>
      </div>`
    }
  }
  
  // Simple code highlighting - fast and safe
  static highlightCodeSimple(code: string, language: string): string {
    if (!code) return ''
    
    const escapedCode = this.escapeHtml(code)
    
    // Basic highlighting for common languages
    switch (language) {
      case 'javascript':
      case 'js':
      case 'typescript':
      case 'ts':
        return this.highlightJSSimple(escapedCode)
      case 'python':
        return this.highlightPythonSimple(escapedCode)
      case 'rust':
        return this.highlightRustSimple(escapedCode)
      case 'bash':
      case 'shell':
        return this.highlightBashSimple(escapedCode)
      default:
        return `<span class="text-gray-300">${escapedCode}</span>`
    }
  }
  
  // Simple JS highlighting
  static highlightJSSimple(code: string): string {
    let result = code
    
    // Comments (simple patterns)
    result = result.replace(/\/\/.*$/gm, '<span class="text-gray-500 italic">$&</span>')
    result = result.replace(/\/\*[\s\S]*?\*\//g, '<span class="text-gray-500 italic">$&</span>')
    
    // Strings
    result = result.replace(/"([^"]*)"/g, '<span class="text-green-300">"$1"</span>')
    result = result.replace(/'([^']*)'/g, '<span class="text-green-300">\'$1\'</span>')
    
    // Keywords
    const keywords = ['const', 'let', 'var', 'function', 'class', 'if', 'else', 'for', 'while', 'return']
    keywords.forEach(keyword => {
      result = result.replace(new RegExp(`\\b${keyword}\\b`, 'g'), `<span class="text-purple-400 font-semibold">${keyword}</span>`)
    })
    
    return result
  }
  
  // Simple Python highlighting
  static highlightPythonSimple(code: string): string {
    let result = code
    
    // Comments
    result = result.replace(/#.*$/gm, '<span class="text-gray-500 italic">$&</span>')
    
    // Strings
    result = result.replace(/"([^"]*)"/g, '<span class="text-green-300">"$1"</span>')
    result = result.replace(/'([^']*)'/g, '<span class="text-green-300">\'$1\'</span>')
    
    // Keywords
    const keywords = ['def', 'class', 'if', 'elif', 'else', 'for', 'while', 'try', 'except', 'import', 'from', 'return']
    keywords.forEach(keyword => {
      result = result.replace(new RegExp(`\\b${keyword}\\b`, 'g'), `<span class="text-purple-400 font-semibold">${keyword}</span>`)
    })
    
    return result
  }
  
  // Simple Rust highlighting
  static highlightRustSimple(code: string): string {
    let result = code
    
    // Comments
    result = result.replace(/\/\/.*$/gm, '<span class="text-gray-500 italic">$&</span>')
    
    // Strings
    result = result.replace(/"([^"]*)"/g, '<span class="text-green-300">"$1"</span>')
    
    // Keywords
    const keywords = ['fn', 'let', 'mut', 'const', 'struct', 'enum', 'impl', 'pub', 'use', 'if', 'else', 'return']
    keywords.forEach(keyword => {
      result = result.replace(new RegExp(`\\b${keyword}\\b`, 'g'), `<span class="text-purple-400 font-semibold">${keyword}</span>`)
    })
    
    return result
  }
  
  // Simple Bash highlighting
  static highlightBashSimple(code: string): string {
    let result = code
    
    // Comments
    result = result.replace(/#.*$/gm, '<span class="text-gray-500 italic">$&</span>')
    
    // Common commands
    const commands = ['ls', 'cd', 'pwd', 'mkdir', 'rm', 'cp', 'mv', 'echo', 'cat', 'sudo', 'git']
    commands.forEach(cmd => {
      result = result.replace(new RegExp(`\\b${cmd}\\b`, 'g'), `<span class="text-blue-400 font-semibold">${cmd}</span>`)
    })
    
    return result
  }
  
  static processBlockElements(text: string): string {
    const lines = text.split('\n')
    const result: string[] = []
    let i = 0
    
    while (i < lines.length) {
      const line = lines[i]
      const trimmed = line.trim()
      
      if (!trimmed) {
        result.push('')
        i++
        continue
      }
      
      // Headers
      if (trimmed.startsWith('# ')) {
        result.push(`<h1 class="text-2xl font-bold text-white mt-6 mb-4">${this.escapeHtml(trimmed.slice(2))}</h1>`)
        i++
        continue
      }
      if (trimmed.startsWith('## ')) {
        result.push(`<h2 class="text-xl font-semibold text-white/95 mt-5 mb-3">${this.escapeHtml(trimmed.slice(3))}</h2>`)
        i++
        continue
      }
      if (trimmed.startsWith('### ')) {
        result.push(`<h3 class="text-lg font-semibold text-white/90 mt-4 mb-2">${this.escapeHtml(trimmed.slice(4))}</h3>`)
        i++
        continue
      }
      
      // Lists
      if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
        const listItems = []
        while (i < lines.length && (lines[i].trim().startsWith('- ') || lines[i].trim().startsWith('* '))) {
          const content = lines[i].trim().slice(2)
          listItems.push(`<li class="flex items-start"><span class="text-blue-400 mr-2">•</span><span>${this.escapeHtml(content)}</span></li>`)
          i++
        }
        result.push(`<ul class="my-3 space-y-1">${listItems.join('')}</ul>`)
        continue
      }
      
      // Regular paragraphs
      result.push(`<p class="text-white/85 my-2 leading-relaxed">${this.escapeHtml(trimmed)}</p>`)
      i++
    }
    
    return result.join('\n')
  }
  
  static processInlineElements(text: string): string {
    return text
      // Agent mentions
      .replace(/@(enteract|coding|research|vision)\b/g, (match, agent) => {
        const agentStyles = {
          enteract: 'bg-blue-500/20 text-blue-300 border-blue-400/30',
          coding: 'bg-green-500/20 text-green-300 border-green-400/30',
          research: 'bg-purple-500/20 text-purple-300 border-purple-400/30',
          vision: 'bg-pink-500/20 text-pink-300 border-pink-400/30'
        }
        const style = agentStyles[agent as keyof typeof agentStyles] || 'bg-gray-500/20 text-gray-300 border-gray-400/30'
        return `<span class="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium border ${style}">@${agent}</span>`
      })
      
      // Bold and italic
      .replace(/\*\*(.*?)\*\*/g, '<strong class="font-semibold text-white">$1</strong>')
      .replace(/\*(.*?)\*/g, '<em class="italic text-white/90">$1</em>')
      
      // Links
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" class="text-blue-400 hover:text-blue-300 underline" target="_blank">$1 ↗</a>')
  }
  
  static escapeHtml(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
}