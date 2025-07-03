# Codex Vault Next-Gen

**An offline-first AI-powered knowledge repository with enterprise-grade performance and premium user experience.**

## 🚀 Overview

Codex Vault Next-Gen is a revolutionary knowledge management application that combines:

- **100% Offline Functionality** - Complete AI and content access without internet dependency
- **Local AI Performance** - Sub-second response times with local LLM inference
- **Premium User Experience** - Modern, minimalistic UI with smooth animations  
- **Cross-Platform Native** - Windows desktop and Android mobile applications
- **Enterprise-Grade Security** - Local-only processing with AES-256 encryption

## 🏗️ Architecture

### Technology Stack

- **Desktop**: Tauri 2.0 + React 18 + TypeScript + Tailwind CSS
- **Mobile**: Kotlin + Jetpack Compose (Android), Swift + SwiftUI (iOS future)
- **Core Library**: Rust with SQLite, Candle AI framework, and FTS5 search
- **AI Models**: Local LLM inference with GGUF/GGML quantized models

### Project Structure

```
CODEX-VAULT-PROJECT/
├── codex-core/              # Rust core library
│   ├── src/
│   │   ├── ai/             # AI inference and embeddings
│   │   ├── db/             # Database operations
│   │   ├── content/        # Content management
│   │   └── update/         # Update system
│   └── migrations/         # Database migrations
├── codex-vault-app/        # Tauri desktop application
│   ├── src/               # React frontend
│   └── src-tauri/         # Rust backend
└── docs/                  # Documentation
```

## 🛠️ Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (18+ LTS)
- [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

### Quick Start

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd CODEX-VAULT-PROJECT
   ```

2. **Install dependencies**
   ```bash
   # Install Rust core dependencies
   cd codex-core
   cargo check
   
   # Install Tauri app dependencies  
   cd ../codex-vault-app
   npm install
   ```

3. **Run development server**
   ```bash
   npm run tauri dev
   ```

### Development Commands

```bash
# Core library
cd codex-core
cargo test                  # Run tests
cargo fmt                   # Format code
cargo clippy               # Lint code

# Tauri application
cd codex-vault-app  
npm run dev                # Frontend dev server
npm run tauri dev          # Full application
npm run tauri build        # Production build
npm run lint               # Lint frontend
npm test                   # Run tests
```

## 🎯 Features

### Content Management
- Multi-format document parsing (PDF, EPUB, Markdown, HTML, JSON, TXT)
- Automatic content indexing with FTS5 full-text search
- AI-enhanced metadata generation (summaries, tags, difficulty assessment)
- Collections and bookmarking system
- Reading progress tracking

### AI Capabilities
- Local LLM inference with <1s response times
- Retrieval-Augmented Generation (RAG) for contextual responses
- Semantic search using vector embeddings
- Content summarization and key point extraction
- Question answering across knowledge base

### User Experience
- Premium, minimalistic interface design
- Dark/light mode with system preference detection
- Smooth animations and micro-interactions
- Keyboard shortcuts for power users
- Responsive design across all screen sizes

### Performance & Security
- SQLite with WAL mode for optimal performance
- AES-256 encryption for all stored data
- Local-only AI processing (no cloud inference)
- Memory-efficient operations (<500MB baseline)
- Automatic database optimization

## 📊 Performance Targets

- **Launch Time**: <2 seconds cold start
- **Search Response**: <200ms for full-text search  
- **AI Response**: <1 second for 90% of queries
- **Memory Usage**: <2GB with AI model loaded
- **Database Size**: 70%+ compression ratio

## 🚢 Deployment

### Desktop Distribution
- Direct download from website with auto-updates
- Code-signed releases for security
- Cross-platform support (Windows, macOS, Linux)

### Mobile Distribution  
- Direct APK download (Phase 1)
- Google Play Store (Phase 2)
- App Store when revenue justifies cost

## 🔒 Privacy & Security

- **Privacy-First**: No data collection without explicit consent
- **Local Processing**: All AI inference happens locally
- **Encryption**: AES-256 for data at rest
- **No Tracking**: Completely offline operation
- **Open Source**: Core components available for security review

## 📈 Roadmap

### Phase 1: Foundation (Months 1-3)
- ✅ Core Rust library architecture
- ✅ SQLite database with FTS5 search
- ✅ Local AI integration framework
- ✅ Tauri desktop application scaffold

### Phase 2: Core Features (Months 4-6)
- [ ] Complete desktop application UI
- [ ] Content import and management
- [ ] AI-powered search and recommendations
- [ ] Android application development

### Phase 3: Polish & Launch (Months 7-12)
- [ ] Performance optimization
- [ ] Premium UI animations
- [ ] Beta testing and feedback integration
- [ ] Public launch and marketing

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust best practices and use `cargo fmt` + `cargo clippy`
- Write comprehensive tests for new functionality
- Update documentation for API changes
- Ensure cross-platform compatibility

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) for the excellent cross-platform framework
- [Candle](https://github.com/huggingface/candle) for Rust-native AI inference
- [SQLite](https://sqlite.org/) for the robust embedded database
- [Anthropic](https://anthropic.com/) for AI development assistance

---

**Built with ❤️ by HANATRA LIMITED**

*Empowering knowledge workers with offline-first AI assistance.*