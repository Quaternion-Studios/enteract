# Embedded Agentic Assistant (Enteract)

<div align="center">

**A next-generation, hyper-personalized desktop AI Computer Use Agent (CUA) and assistant that seamlessly blends web, native, and 3D interfaces**


[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue?logo=tauri)](https://tauri.app/)
[![Vue 3](https://img.shields.io/badge/Vue.js-3.x-green?logo=vue.js)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-latest-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[![Windows](https://img.shields.io/badge/Windows-10%2F11-blue?logo=windows)](https://www.microsoft.com/windows)


[🌐 **Try Enteract**](https://www.tryenteract.com/) | [📚 Documentation](./resources/) | [🐛 Report Issues](../../issues) | [💬 Discussions](../../discussions)

<img width="600" alt="Embedded Agentic Assistant Interface" src="https://github.com/user-attachments/assets/ebcb93f3-53b5-420c-91f6-c586d53c7ea3" />


</div>

### Core Features

- **Advanced Speech Recognition** - Real-time transcription with Whisper integration
- **Multi-Modal AI** - Vision analysis, document understanding, and conversational intelligence
- **System Integration** - OS-level automation, screenshot capture, and application control
- **Personal Knowledge Base** - RAG system with document embedding and semantic search
- **Beautiful UI** - Frameless windows with 3D visuals, glassmorphism effects, and smooth animations
- **High Performance** - Rust backend with optimized data storage (JSON → SQLite migration)

### Platform Support

**Windows**
- **Windows 10** (1903 or later) - Fully supported
- **Windows 11** - Fully supported and optimized
- Advanced features: Eye tracking, audio loopback, system automation
- Native Windows API integration for seamless OS interaction

### Technical Architecture

- **Hybrid Storage System** - Seamless migration from JSON to SQLite with zero downtime
- **Modular AI Agents** - Specialized agents for different tasks (coding, research, conversation)
- **Cross-Platform** - Windows, macOS, and Linux support via Tauri
- **Audio Processing** - Loopback capture, noise reduction, and speech-to-text
- **Comprehensive Testing** - Full test suite with Vitest and Vue Test Utils

## Quick Start

### System Requirements

**Minimum Requirements:**
- **Windows:** Windows 10 (build 1903+) or Windows 11
- **RAM:** 4GB minimum, 8GB recommended
- **Storage:** 8GB available space
- **Camera:** Any USB or integrated camera (for eye tracking)
- **Microphone:** Any audio input device (for speech recognition)

**Development Requirements:**
- **Node.js** 18+ 
- **Rust** (latest stable)
- **Platform-specific build tools:**
  - **Windows:** Visual Studio Build Tools 2019/2022 or Visual Studio Community

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd enteract

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### First Launch

1. **Configure AI Models** - Set up your preferred Ollama models or OpenAI API keys
2. **Calibrate Eye Tracking** - Follow the brief calibration process for gaze controls
3. **Explore Features** - Open different windows and try voice commands

## Project Structure

```
embedded-agentic-assistant/
├── src/                           # Vue 3 + TypeScript frontend
│   ├── components/                # UI components
│   │   ├── ControlPanel.vue       # Main control interface
│   │   ├── ChatWindow.vue         # AI chat interface
│   │   └── ConversationalWindow.vue # Voice interaction UI
│   ├── composables/               # Vue composables
│   │   ├── useEyeTracking.ts      # Eye tracking system
│   │   ├── useSpeechTranscription.ts # Speech recognition
│   │   └── useWindowManager.ts    # Window management
│   ├── types/                     # TypeScript definitions
│   └── tests/                     # Comprehensive test suite
├── src-tauri/                     # Rust backend
│   ├── src/
│   │   ├── ai_commands.rs         # AI model integration
│   │   ├── data/                  # Storage system
│   │   │   ├── json_store.rs      # Legacy JSON storage
│   │   │   ├── sqlite_store.rs    # Modern SQLite storage
│   │   │   ├── migration.rs       # Migration utilities
│   │   │   └── hybrid_store.rs    # Auto-selecting storage
│   │   ├── rag_system.rs          # Document embedding & search
│   │   ├── speech.rs              # Whisper integration
│   │   └── screenshot.rs          # Screen capture
│   └── capabilities/              # Tauri permissions
└── resources/                     # Documentation & assets
```

## Testing & Quality Assurance (needs contributors)

<img width="794" height="575" alt="image" src="https://github.com/user-attachments/assets/fa685380-4156-4a8e-ab1c-30a28cb20194" />

Long term TDD is intended, if you have rust or UX testing expertise please contribute!

## Configuration & Setup

### AI Models

Configure your AI models in the settings:

- **Ollama** - Local models for privacy-focused AI
- **OpenAI / DeepSeek API** - Cloud-based models for advanced capabilities (pending)
- **Whisper** - Local speech recognition

## Contributing

We welcome contributions! Whether you're fixing bugs, adding features, or improving documentation, your help makes this project better. Make sure to review the [Contributing Guide](https://github.com/Quaternion-Studios/enteract/blob/main/CONTRIBUTING.md) first (short).

### Development Setup

1. **Fork and Clone** the repository
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Install dependencies** (`npm install`)
4. **Run tests** (`npm run test`) to ensure everything works
5. **Make your changes** with proper test coverage
6. **Commit your changes** (`git commit -m 'Add amazing feature'`)
7. **Push to the branch** (`git push origin feature/amazing-feature`)
8. **Open a Pull Request**

### Areas for Contribution

- **UI/UX Improvements** - Enhanced visual design and user experience
- **AI Capabilities** - New AI agents and improved prompts
- **Integrations** - Connect with more external services
- **Documentation** - Tutorials, guides, and API docs
- **Testing** - Expanded test coverage and performance benchmarks
- **Windows Features** - Advanced Windows-specific integrations
  
(Experimental)
- **Eye Tracking** - Better calibration and gaze accuracy (experimental)
- **Platform Support** - macOS/Linux compatibility


### Code Style

- **TypeScript/Vue** - Use Composition API with TypeScript
- **Rust** - Follow standard Rust conventions with `rustfmt`
- **Tests** - Write tests for new features and bug fixes
- **Documentation** - Update relevant documentation

## License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Rust Community** - For the powerful systems language and crate development
- **Tauri Team** - For the amazing cross-platform framework
- **Vue.js Community** - For the reactive frontend framework
- **Whisper** - For speech recognition technology

## Roadmap

### Phase 1:
- [x] Core UI components and window management
- [x] Speech recognition integration
- [x] SQL Based RAG
- [ ] Computer Use Agent (CUA) MCP - work in progress currently just using Regex

### Phase 2:
- [ ] Cloud integration (OAI API, Deepseek API, Azure AI)
- [ ] Enhanced AI agent capabilities (CUA)
- [ ] Basic eye tracking implementation w/ model context
- [ ] Improved RAG system with better embeddings / file ref
- [ ] Multi-modal AI interactions

### Phase 3:
- [ ] Multi-platform support
- [ ] Advanced automation workflows + CUA
- [ ] Plugin system for extensibility

## Support & Community

- **Website** - Visit [tryenteract.com](https://www.tryenteract.com/) for more information
- **Issues** - Report bugs and request features on [GitHub Issues](../../issues)
- **Discussions** - Join conversations on [GitHub Discussions](../../discussions)
- **Documentation** - Check the `/resources` directory for detailed guides

---

<div align="center">

**Star ⭐ this repository if you find it helpful!**

Made with ❤️ by the community

Started by [Rohan](https://github.com/rohxnsxngh) and [Chase](https://github.com/MC-Meesh)
</div>
