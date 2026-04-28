# nsh — The Agentic Shell

nsh is an **agentic terminal shell** for Linux that combines the power of a traditional Unix shell (like zsh or bash) with built-in AI capabilities. It runs entirely in your terminal, understands your system context, and can perform agentic tasks using LLMs — all without leaving the command line.

Think of it as **zsh with a brain**: you get all the familiar shell ergonomics (commands, pipes, history, completions) plus an AI layer that can reason about your system, search the web, read files, and execute tasks on your behalf.

---

## What Makes nsh Agentic

Unlike a typical AI chatbot wrapper, nsh is designed to **act** on your Linux system:

- **System-aware**: Knows your current directory, file structure, and command history
- **Tool-equipped**: Built-in tools for `web_search`, `cat`, `ls`, and `grep` that the AI can invoke
- **RAG-ready**: Document indexing and semantic search via vector embeddings (Ollama-powered)
- **Provider-agnostic**: Works with Ollama (local), OpenAI, Anthropic, or any OpenAI-compatible API
- **Terminal-native**: Full TUI built with [ratatui](https://github.com/ratatui/ratatui) — no browser, no electron

---

## Features

### Core Shell
- **Rich TUI**: Full-screen terminal interface with syntax highlighting and color-coded output
- **Smart Completions**: Tab-based autocomplete for commands (from `$PATH`), files, directories, and command history
- **Command History**: Scrollable history with Up/Down navigation
- **Mouse Support**: Scroll history, navigate suggestions, and click settings items
- **Kill Ring**: Bash-style `Ctrl+W` (cut word) and `Ctrl+Y` (yank/paste)
- **System Clipboard**: `Ctrl+Shift+C` to copy, `Ctrl+Shift+V` to paste
- **Word Navigation**: `Alt+←/→` to move by word, `Ctrl+A/E` for line start/end

### AI Integration
- **Multi-provider**: Ollama, OpenAI, Anthropic, OpenAI-Compatible endpoints
- **Dynamic Model Discovery**: Automatically fetches available models from Ollama; hardcoded defaults for cloud providers
- **Full-screen Settings TUI**: Configure provider, model, base URL, and API key via `settings` or `/settings`
  - Stack-based navigation: select a setting → full-screen sub-page → `Esc` to go back
  - Click any option with your mouse to navigate
- **Persistent Config**: Settings saved to `~/.config/nsh/config.json`

### Agent Tools
The AI layer has access to structured tools for interacting with your system:

| Tool | Description |
|------|-------------|
| `web_search` | Search the web via DuckDuckGo |
| `cat` | Read file contents |
| `ls` | List directory contents |
| `grep` | Search for patterns in files |

### RAG (Retrieval-Augmented Generation)
- **Document Indexing**: Index documents into a local vector store for semantic retrieval
- **Ollama Embeddings**: Uses Ollama's `/api/embeddings` endpoint for local embedding generation
- **Semantic Search**: Query indexed documents by meaning, not just keywords

---

## Installation

### Requirements
- Rust (latest stable)
- Cargo
- For local AI: [Ollama](https://ollama.com) running on `localhost:11434`
- For cloud AI: API key from OpenAI, Anthropic, or compatible provider

### Build

```bash
git clone <repo-url>
cd nsh
cargo build --release
```

The binary will be at `./target/release/nsh`.

---

## Usage

### Start the Shell

```bash
./target/release/nsh
```

You'll see the welcome screen:
```
Welcome to nsh - AI-Powered Shell
Type 'help' for commands
Use Tab for autocomplete, Up/Down for history
Type 'settings' or press Ctrl+, for AI settings
```

### Commands

```bash
# Standard shell commands work as expected
nsh: ~$ ls -la
nsh: ~$ cd /path/to/dir
nsh: ~$ cat file.txt
nsh: ~$ clear
nsh: ~$ exit

# AI commands (when AI is enabled)
nsh: ~$ ask "what is the largest file in this directory?"
nsh: ~$ do "find all .rs files and count lines"
nsh: ~$ plan "set up a rust web server"
nsh: ~$ build "a REST API in Python with FastAPI"

# Open settings TUI
nsh: ~$ settings
nsh: ~$ /settings        # slash command alias
```

### Settings TUI

Open settings with `settings`, `/settings`, or `Ctrl+Comma`:

1. **Home Page**: Shows current AI configuration (Provider, Model, Base URL, API Key, Enable)
   - Navigate with `↑`/`↓`, select with `Enter`
   - **Save**: persists to `~/.config/nsh/config.json`
   - **Cancel**: closes without saving
2. **Sub-pages**: Each setting opens a dedicated full-screen page:
   - **Provider**: Choose between Ollama, OpenAI, Anthropic, OpenAI Compatible
   - **Model**: Select from available models (auto-fetched for Ollama)
   - **Base URL**: Edit the API endpoint URL
   - **API Key**: Enter your API key (masked as `••••••••`)
   - **Enable**: Toggle AI on/off
3. **Navigation**: `Esc` goes back one page; on Home it closes settings
4. **Mouse**: Click any row to select and open it

---

## Key Bindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate history or suggestions |
| `Tab` | Autocomplete command/file |
| `Ctrl+C` | Interrupt / clear input |
| `Ctrl+D` | EOF (exit when input is empty) |
| `Ctrl+A` | Move cursor to line start |
| `Ctrl+E` | Move cursor to line end |
| `Ctrl+W` | Delete word before cursor (kill) |
| `Ctrl+Y` | Yank (paste) last killed text |
| `Ctrl+U` | Delete from cursor to line start |
| `Ctrl+K` | Delete from cursor to line end |
| `Alt+←` / `Alt+→` | Move cursor by word |
| `Ctrl+Shift+C` | Copy current input to clipboard |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+,` | Open AI Settings |
| `Esc` | Cancel suggestions / close dialogs |
| `Mouse Scroll` | Scroll history or suggestions |
| `Mouse Click` | Select settings items |

---

## Configuration

Config is stored at `~/.config/nsh/config.json`:

```json
{
  "ai": {
    "provider": "Ollama",
    "model": "llama3.2:latest",
    "base_url": "http://localhost:11434",
    "api_key": null,
    "enabled": false
  },
  "rag": {
    "embed_model": null,
    "collection_name": ""
  }
}
```

| Provider | Default Base URL |
|----------|-----------------|
| Ollama | `http://localhost:11434` |
| OpenAI | `https://api.openai.com/v1` |
| Anthropic | `https://api.anthropic.com` |
| OpenAI Compatible | `http://localhost:11434/v1` |

---

## Architecture

```
nsh/
├── src/
│   ├── main.rs              # Entry point, event loop, terminal init
│   ├── lib.rs               # Public API exports
│   ├── ai/
│   │   ├── mod.rs           # AiProvider, chat(), fetch_models()
│   │   └── config.rs        # ProviderType, AiConfig
│   ├── modules/
│   │   ├── commands.rs      # Built-in commands + external process spawning
│   │   ├── completions.rs   # Tab autocomplete (PATH, files, history)
│   │   ├── config.rs        # UI color constants
│   │   ├── keybindings.rs   # Keyboard shortcuts, Action enum, clipboard
│   │   ├── render.rs        # Ratatui rendering (shell + settings pages)
│   │   └── state.rs         # App state, Entry, SettingsState, SettingsPage
│   ├── rag/
│   │   └── mod.rs           # RagEngine: document indexing + semantic search
│   ├── storage/
│   │   ├── local.rs         # LocalStorage, NshConfig (JSON persistence)
│   │   └── vector.rs        # VectorStore stub (Qdrant integration scaffold)
│   └── tools/
│       ├── mod.rs           # Tool definitions + execute_tool() dispatcher
│       ├── terminal.rs      # cat, ls, grep implementations
│       └── web_search.rs    # DuckDuckGo web search
├── Cargo.toml
└── README.md
```

### Stack-based Settings Navigation

The settings TUI uses a navigation stack (`Vec<SettingsPage>`) to manage sub-pages:

```
Home ──Enter──▶ Provider ──Enter──▶ back to Home (provider set)
  ├── Enter──▶ Model ──Enter──▶ back to Home (model set)
  ├── Enter──▶ BaseUrl ──Enter──▶ back to Home
  ├── Enter──▶ ApiKey ──Enter──▶ back to Home
  └── Enter──▶ Enable ──Enter──▶ back to Home

Esc on Home ──▶ close settings
Esc on sub-page ──▶ pop back to Home
```

---

## Roadmap

- [x] Full TUI shell with ratatui
- [x] Settings TUI with provider/model/api-key configuration
- [x] Slash command aliases (`/settings`, `/help`, etc.)
- [x] Tool system (web_search, cat, ls, grep)
- [x] RAG scaffolding (document indexing + semantic search)
- [ ] Wire AI commands (`ask`, `do`, `plan`, `build`) to actual LLM calls
- [ ] Streaming AI responses
- [ ] Multi-turn conversation memory
- [ ] File watching / auto-indexing for RAG
- [ ] Plugin system for custom tools
- [ ] Configuration file editing (beyond AI settings)

---

## License

MIT
