# nsh Architecture

**nsh** (agentic shell) is a terminal-native Linux shell with a ratatui TUI, Unix-style command execution, and an LLM agent layer (ask / do / plan / build) that can call filesystem and web tools plus optional RAG.

This document is a snapshot of the **current** codebase (not the README roadmap). crate `nsh` `0.1.0`, Rust edition `2024`. Binary and library share `src/`.

**Mermaid diagrams** (GitHub / VS Code / most Markdown previews render them):

- HLD: [process](#12-process-model), [layers](#13-layered-architecture), [AI sequence](#19-runtime-data-flow-ai-verb)
- LLD: [call graph](#5-call-graph-who-uses-what)
- Full set in [§8](#8-detailed-mermaid-diagrams): context, modules, event loop, command router, two execution paths, completions, ReAct, tool parser/dispatcher, RAG, settings states, `App` class, keys, render, providers, threads, disk, startup, errors

---

## 1. High-Level Design (HLD)

### 1.1 Purpose

Give the user a familiar interactive shell (history, completions, `cd`, spawn external programs) **plus** first-class AI verbs that can reason, search, and mutate files without leaving the TUI.

### 1.2 Process model

The binary is **sync** at the UI layer. Async work (LLM, embeddings, HTTP search) is `tokio::runtime::Runtime::new().block_on(...)` either on the main thread (settings model fetch, `/index`) or on a spawned OS thread (AI verbs) so the spinner can keep painting.

```mermaid
flowchart TB
  subgraph proc["nsh process — src/main.rs"]
    direction TB
    TERM["crossterm: raw mode<br/>alternate screen<br/>bracketed paste"]
    LOOP["Event loop"]
    APP["App state<br/>modules/state.rs"]
    DRAW["ratatui render<br/>modules/render.rs"]
    TERM --> LOOP
    LOOP -->|"poll 50ms"| DRAW
    DRAW --> APP
    LOOP --> APP

    LOOP -->|"ls / git / cd / help"| CMD["execute_command()"]
    LOOP -->|"index / /index"| IDX["RagEngine on UI thread"]
    LOOP -->|"ask do plan build"| AI["OS thread + tokio Runtime<br/>run_ai_command()"]
  end

  CMD --> OSBIN["OS binaries via std::process::Command"]
  IDX --> DISK["~/.config/nsh/<br/>config.json<br/>rag/nsh_rag.json"]
  AI --> DISK
  AI --> LLM["Ollama / OpenAI-compat<br/>chat + /api/embeddings"]
  AI --> DDG["DuckDuckGo Instant Answer"]
  LOOP --> DISK
```

### 1.3 Layered architecture

| Layer | Crate path | Responsibility |
|-------|------------|----------------|
| **Presentation** | `src/main.rs`, `modules/render.rs`, `modules/keybindings.rs` | Terminal lifecycle, input, settings UI, shell paint |
| **Application state** | `modules/state.rs` | History, input buffer, settings stack, AI loading |
| **Shell runtime** | `modules/commands.rs`, `modules/completions.rs` | Built-ins, process spawn, tab complete |
| **Agent** | `ai/agent.rs` | ReAct loop: prompt → parse `TOOL:`/`ARGS:` → tools → answer |
| **LLM adapter** | `ai/mod.rs`, `ai/config.rs` | Provider enum, keys, `aisdk` OpenAI-compatible chat |
| **Tools** | `tools/*` | Structured FS + web_search for the agent (not the user shell) |
| **RAG** | `rag/mod.rs` | Embed via Ollama, index/search documents |
| **Persistence** | `storage/local.rs`, `storage/vector.rs` | JSON config + in-process cosine vector file |

```mermaid
flowchart TB
  subgraph L1["1 Presentation"]
    MAIN["main.rs"]
    REND["modules/render.rs"]
    KEYS["modules/keybindings.rs"]
  end
  subgraph L2["2 Application state"]
    ST["modules/state.rs — App"]
  end
  subgraph L3["3 Shell runtime"]
    CMD["modules/commands.rs"]
    COMP["modules/completions.rs"]
  end
  subgraph L4["4 Agent"]
    AG["ai/agent.rs"]
  end
  subgraph L5["5 LLM adapter"]
    AIP["ai/mod.rs AiProvider"]
    AIC["ai/config.rs"]
  end
  subgraph L6["6 Tools"]
    TM["tools/mod.rs execute_tool"]
    TT["tools/terminal.rs"]
    TW["tools/web_search.rs"]
  end
  subgraph L7["7 RAG"]
    RAG["rag/mod.rs RagEngine"]
  end
  subgraph L8["8 Persistence"]
    LS["storage/local.rs"]
    VS["storage/vector.rs"]
  end

  MAIN --> REND
  MAIN --> KEYS
  MAIN --> ST
  MAIN --> CMD
  MAIN --> AG
  KEYS --> ST
  REND --> ST
  COMP --> ST
  AG --> AIP
  AG --> TM
  AG --> RAG
  AIP --> AIC
  TM --> TT
  TM --> TW
  RAG --> VS
  MAIN --> LS
  AG --> LS
```

### 1.4 Two execution paths (important)

User-typed Unix commands and agent tools are **not the same code**:

1. **Human shell path** — `execute_command` uses `std::process::Command` (real `ls`, `git`, …) plus a few builtins (`cd`, `clear`, `help`, `settings`).
2. **Agent tool path** — `tools::execute_tool` runs in-process Rust (`cat`, `ls`, `grep`, `write_file`, …) and DuckDuckGo. The model never gets a free-form shell; it only gets named tools.

### 1.5 AI command semantics

| Verb | Max ReAct steps | Intent |
|------|-----------------|--------|
| `ask` / `/ask` | 1 | Q&A; at most one tool call |
| `plan` / `/plan` | 2 | Numbered plan; read-only tools preferred |
| `do` / `/do` | 3 | Small FS tasks (write/copy/move/delete/mkdir) |
| `build` / `/build` | 6 | Explore then write files |

`index` / `/index` is **not** an `AiCommand`; it walks files and calls `RagEngine::index_document` (max 50 files, non-recursive except the immediate directory).

### 1.6 Providers

All chat goes through `aisdk::providers::OpenAICompatible` with a base URL + API key. Provider type only changes:

- default URL
- whether a key is required (Ollama: no)
- env-var fallbacks
- model list (`/api/tags` for Ollama; hardcoded lists otherwise)

Embeddings always hit `{embed_base}/api/embeddings` (Ollama-style), even if chat is a cloud provider. Non-Ollama chat still defaults embed base to `http://localhost:11434`.

### 1.7 Persistence layout

```
~/.config/nsh/
  config.json          # NshConfig { ai, rag }
  rag/nsh_rag.json     # VectorStore points (vectors + payloads)
```

`qdrant-client` is a Cargo dependency but **not used**. `VectorStore` is a JSON file + cosine similarity in memory.

### 1.8 External dependencies (role)

| Crate | Use |
|-------|-----|
| `crossterm` | Raw mode, events, mouse, paste, alternate screen |
| `ratatui` | TUI widgets / layout |
| `arboard` | System clipboard + Linux primary selection |
| `aisdk` | LLM request (`LanguageModelRequest` + OpenAI-compatible provider) |
| `tokio` | Async runtime for HTTP / embeddings / agent |
| `reqwest` | Ollama tags/embeddings, DuckDuckGo |
| `serde` / `serde_json` | Config, tool JSON, vector persist |
| `dirs` | OS config directory |
| `regex` | Agent `grep` |
| `urlencoding` | Search query |
| `thiserror` / `log` / `async-trait` | Errors; `log`/`async-trait` unused in src today |
| `qdrant-client` | Declared; unused in src |

### 1.9 Runtime data flow (AI verb)

```mermaid
sequenceDiagram
  actor User
  participant Main as main.rs UI thread
  participant App as App + render
  participant Thr as spawned OS thread
  participant Rt as tokio Runtime
  participant Ag as run_ai_command
  participant Prov as AiProvider.chat
  participant Rag as RagEngine
  participant Tools as execute_tool
  participant LLM as OpenAI-compat HTTP
  participant Emb as Ollama /api/embeddings

  User->>Main: Enter "do copy a to b"
  Main->>Main: AiCommand::from_str("do")
  Main->>App: ai_loading = Asking/Doing/…
  Main->>App: render spinner
  Main->>Thr: spawn + mpsc
  Thr->>Rt: Runtime::new().block_on
  Rt->>Ag: run_ai_command(Do, query, AiConfig, storage)

  alt AI disabled
    Ag-->>Main: Err NotEnabled
  else enabled
    Ag->>Prov: create_provider
    Ag->>Rag: new_from_config best-effort
    Rag->>Emb: embed query
    Emb-->>Rag: Vec f32
    Rag-->>Ag: retrieve_context k=3
    loop up to max_steps Ask=1 Do=3 Plan=2 Build=6
      Ag->>Prov: chat joined history string
      Prov->>LLM: generate_text
      LLM-->>Prov: model text
      alt parse_tool_call found TOOL/ARGS
        Ag->>Tools: execute_tool name args
        Tools-->>Ag: JSON Observation
      else no tool call
        Ag-->>Thr: Ok display lines
      end
    end
  end

  loop UI while mpsc empty
    Main->>App: frame++ and render spinner
    Main->>Main: drain leftover input events
  end
  Thr-->>Main: mpsc Result
  Main->>App: ai_loading = None
  Main->>App: add_entry Output or System error
```

---

## 2. Low-Level Design (LLD)

### 2.1 Crate layout

```
nsh/
├── Cargo.toml / Cargo.lock
├── README.md
├── architect.md          # this file
├── todo.md               # scratch notes (untracked)
├── src/
│   ├── main.rs           # binary: event loop
│   ├── lib.rs            # crate root + re-exports
│   ├── ai/
│   ├── modules/          # TUI / shell
│   ├── rag/
│   ├── storage/
│   └── tools/
└── target/               # cargo build artifacts (not source)
```

`[lib] name = "nsh"` + `[[bin]] name = "nsh"`: the binary `use nsh::{...}` against the library.

### 2.2 Event loop (`main.rs`)

1. `enable_raw_mode`, `EnterAlternateScreen`, `EnableBracketedPaste`.
2. **No mouse capture** in shell mode (so the emulator can drag-select). Capture is toggled **on** only while `app.show_settings`.
3. Each tick: `render` → `poll(50ms)` → handle Key / Paste / Mouse.
4. Alt keys: many terminals send `Esc` then a key. `resolve_key_with_alt_meta` peeks 25ms and ORs `ALT`.
5. Shell keys map through `get_action` → either custom match in `main` (Execute, history, complete, settings, interrupt, EOF) or `execute_action` (editing / clipboard).
6. Settings keys are handled separately (`handle_settings_input`) and do **not** use `Action`.
7. On exit: disable mouse, disable raw, `DisableBracketedPaste`, `LeaveAlternateScreen`.

**Sentinel outputs from `execute_command`:**

- `"__SETTINGS__"` → open settings (same as `Ctrl+,`)
- `"__CLEAR__"` → `app.clear()`

AI verbs are intercepted **before** `execute_command` so they never hit the stub branch in `commands.rs` in the live binary.

### 2.3 `App` state machine

```
entries: Vec<Entry>          # Command | Output | System
current_input + cursor_position
scroll_offset / total_lines  # total_lines updated from wrapped rows in render
suggestions + selected + scroll
history_index + saved_input  # Up/Down through Command entries
kill_ring                    # Ctrl+W / Ctrl+U / Ctrl+K → Ctrl+Y
show_settings + settings_nav stack + SettingsState
ai_loading: Option<AiLoadingState>
```

```mermaid
flowchart TB
  subgraph app["App"]
    E["entries: Command Output System"]
    I["current_input + cursor_position byte index"]
    SC["scroll_offset / total_lines wrapped"]
    SG["suggestions + selected + scroll"]
    H["history_index + saved_input"]
    K["kill_ring cap 100"]
    S["show_settings + settings_nav + SettingsState"]
    L["ai_loading spinner"]
  end
  E -->|"get_history_commands"| H
  I -->|"update_suggestions"| SG
  K -->|"Ctrl+Y yank"| I
  S -->|"Esc pop empty"| SHELL["show_settings false"]
```

Settings navigation is a stack (`Vec<SettingsPage>`). Empty stack + `show_settings` means Home. Esc pops; empty stack closes settings.

Home fields (cursor index): Provider, Model, BaseUrl, ApiKey, Enable, Save, Cancel.

Save writes `~/.config/nsh/config.json` via `LocalStorage`. Cancel / Esc on Home discard in-memory edits (not auto-saved).

### 2.4 Input editing (LLD)

- Cursor is a **byte** index; `prev_char_boundary` / `next_char_boundary` keep UTF-8 valid.
- Word ops are whitespace-delimited (`word_start_backward` / `word_start_forward`).
- Kill ring: insert at front, cap 100. Yank pastes first entry (not a rotating ring).
- Copy (`Ctrl+Shift+C`): if input non-empty, copy word around cursor (or whole line if cursor at end); if empty, last `EntryType::Output` joined by `\n`.
- Paste: `Event::Paste` (bracketed) preferred; also arboard + Linux primary selection.

### 2.5 Completions (LLD)

`PATH_COMMANDS`: lazy unique basenames of files in `$PATH` dirs (not execute-bit filtered).

`update_suggestions`:

- empty input → hide
- `cd ` prefix → directories under parsed path (`parse_dir_path`)
- else: builtin AI/slash names, then PATH prefix match, then history commands (cap ~20)

Tab: if list already shown, insert selected; else compute; if exactly one, auto-insert.

### 2.6 Command execution (LLD)

`execute_command`:

- `cd` → `std::env::set_current_dir` (process-global cwd for later commands)
- `clear` / `settings` / `help` / `exit` builtins
- AI names in this module return a routing stub string (normally unused because `main` intercepts)
- else `Command::new(program).args(args).output()` — **no pipes, no redirection, no job control, no glob expansion**
- `ls`/`dir`/`eza`/… without a format flag get `-1` prepended so the TUI gets one name per line
- `normalize_output_lines` splits `\r`/`\n` and tab-separated columns

### 2.7 Agent loop (LLD)

`run_ai_command`:

1. Fail if `!ai_config.enabled`.
2. `create_provider` — fails `MissingApiKey` if provider requires key and resolved key length `< 8`.
3. RAG init best-effort; `retrieve_context` concatenates up to 3 docs (hard stop after index 2).
4. History vector of strings (not chat-role API): System prompt, tools JSON, RAG, `cwd`, User.
5. Each step: `provider.chat(vec![history.join("\n\n")])` — **entire transcript as one prompt string**.
6. `parse_tool_call`:
   - `TOOL: name` + `ARGS: {json}` (first line, or remaining as JSON)
   - or a JSON object with `name`/`arguments` or `tool`/`args`
7. Tool result appended as `Observation (tool=…): …`. Errors become `{"error": …}` JSON, not loop abort.
8. If all steps used without a non-tool reply, a fallback English sentence is shown.

`AiProvider::chat` joins messages with `\n` and calls `generate_text()`. No streaming, no native tool-calling API.

### 2.8 Tools (LLD)

Dispatcher `execute_tool(name, args)`:

| Name | Implementation | Notes |
|------|----------------|-------|
| `web_search` | DuckDuckGo Instant Answer JSON | Abstract + up to 5 related topics |
| `cat` | `read_to_string` | UTF-8 files only |
| `ls` | `read_dir` | dirs first, then name sort |
| `grep` | regex; files or dirs | dir: max depth 3, text-ish extensions only |
| `write_file` | create parents + write | overwrite |
| `delete_path` | file or `remove_dir_all` | **recursive, no trash** |
| `copy_path` / `move_path` | recursive copy / rename | |
| `mkdir` | `create_dir_all` | |

No sandbox, no path allowlist, no confirmation. Agent tools operate on the real filesystem relative to process cwd.

`index_directory_simple` (main.rs): **non-recursive** directory listing (or a single file), extensions `rs toml md txt json yaml yml py js ts go sh`, max 50 files.

### 2.9 Vector store (LLD)

`StoredPoint { vector: Vec<f32>, payload: HashMap }` in `rag/nsh_rag.json`.

Search: cosine similarity, sort descending, take `limit`. IDs are **array indices**, so they shift if the file is rewritten as a list (search still works via payload).

Embed request body: `{ "model", "prompt" }` → `data["embedding"]` array of f64 → f32.

### 2.10 Render (LLD)

Two roots: `render_shell` vs `render_settings`.

**Shell**

- Vertical layout: output list (`Min(1)`), optional 1-row yellow loading bar, 3-row input.
- `build_display_rows` wraps to pane width (whitespace then hard wrap).
- Command rows: `shorten_cwd(cwd)$ cmd` with green cwd + gray command.
- Scroll: if previously at bottom, stay pinned after wrap-height change.
- Suggestions overlay: 40 cols, above input, selected row blue.
- Real cursor via `set_cursor_position`; hidden while `ai_loading`.

**Settings pages** are full-screen bordered blocks with green titles; Home / Provider / Model / Enable are lists; BaseUrl / ApiKey are text fields (API key masked with bullets).

### 2.11 Config schema

```json
{
  "ai": {
    "provider": "ollama",
    "model": "llama3",
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

`ProviderType` serializes lowercase. `enabled` defaults false if missing. `RagConfig.embed_model` is stored but `RagEngine::new_from_config` currently prefers `ai_config.model` then `"nomic-embed-text"` — **`rag.embed_model` is not read**.

API key resolution order: trimmed config key → provider env vars (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`/`GOOGLE_API_KEY`, `OPENROUTER_API_KEY`, plus `NSH_API_KEY`).

### 2.12 Known gaps vs README / intended design

- No pipes, quotes, glob, or job control in the human shell.
- Chat is single-shot prompt, not a multi-turn role array; no conversation memory across commands.
- No streaming.
- `qdrant-client` unused; RAG is local JSON.
- `index` is shallow (one directory level).
- Agent mutating tools have no confirmation UI.
- README still lists “Wire AI commands” as unchecked; they **are** wired via `ai/agent.rs`.

---

## 3. Folder and file inventory

### 3.1 Repository root

| Path | Use |
|------|-----|
| `Cargo.toml` | Package metadata, lib+bin targets, dependencies |
| `Cargo.lock` | Pinned dependency graph |
| `README.md` | User-facing product doc, keys, config, older architecture sketch |
| `architect.md` | This HLD/LLD inventory |
| `todo.md` | Informal notes (e.g. `do` should show final output, not a summary) |
| `src/` | All application source |
| `target/` | rustc/cargo output (`debug/nsh`, incremental, deps). Do not edit |

### 3.2 `src/`

| File | Use |
|------|-----|
| `src/lib.rs` | Declares `ai`, `modules`, `storage`, `tools`, `rag`; re-exports public types used by the binary |
| `src/main.rs` | Process entry: terminal setup, event loop, settings I/O, AI loading animation, `/index` walker |

### 3.3 `src/ai/`

| File | Use |
|------|-----|
| `mod.rs` | `AiError`, model lists, `fetch_models`, `AiProvider` (aisdk chat), `create_provider` |
| `config.rs` | `AiConfig`, `ProviderType` (URLs, env keys, serde), `ConfigError` |
| `agent.rs` | `AiCommand`, system prompts, ReAct `run_ai_command`, `parse_tool_call` |

### 3.4 `src/modules/`

| File | Use |
|------|-----|
| `mod.rs` | Submodule decls; re-export `Action` / keybinding helpers |
| `state.rs` | `App`, history `Entry`, settings enums/state, kill-ring and word ops, loading spinner |
| `render.rs` | Shell + all settings pages, wrapping, suggestion popup |
| `keybindings.rs` | `KeyBindings` table, `get_action`, clipboard, `execute_action` |
| `commands.rs` | `execute_command`, cwd shorten, ls `-1` tweak, output line split |
| `completions.rs` | `PATH_COMMANDS`, `update_suggestions` |
| `config.rs` | UI colors and sizes (`MAX_VISIBLE_SUGGESTIONS`, `PROMPT_TEXT`, …) |

### 3.5 `src/tools/`

| File | Use |
|------|-----|
| `mod.rs` | `ToolDefinition` JSON schema for the LLM, `get_tool_definitions`, `execute_tool` match |
| `terminal.rs` | In-process FS tools for the agent |
| `web_search.rs` | DuckDuckGo Instant Answer client |

### 3.6 `src/storage/`

| File | Use |
|------|-----|
| `mod.rs` | Re-exports local + vector types |
| `local.rs` | `~/.config/nsh`, generic JSON load/save, `NshConfig` / `RagConfig` |
| `vector.rs` | File-backed cosine `VectorStore` (Qdrant-shaped API, no Qdrant) |

### 3.7 `src/rag/`

| File | Use |
|------|-----|
| `mod.rs` | `RagEngine`, `Document`, `RetrievedDocument`, Ollama embeddings, index + search |

---

## 4. Snippet inventory (types, functions, constants)

Grouped by file. Private helpers are included so the map is complete.

### 4.1 `src/main.rs`

| Symbol | Role |
|--------|------|
| `resolve_key_with_alt_meta` | Treat Esc+key as Alt+key |
| `load_settings_state` | Config → `SettingsState` |
| `save_settings_state` | `SettingsState` → `config.json` |
| `set_mouse_capture` | Enable/disable mouse for settings clicks |
| `main` | Terminal lifecycle + event loop |
| `settings_apply_paste` | Paste into Base URL / API key fields |
| `handle_settings_input` | Keys while settings is open |
| `settings_handle_enter` | Push/pop settings pages; fetch models on provider change |
| `index_directory_simple` | Shallow RAG index used by `index` command |

### 4.2 `src/lib.rs`

Re-exports only (no logic). Public surface: AI types, `run_ai_command`, `execute_command`, UI constants, `render`, `App`/`Entry`, RAG, storage, tools.

### 4.3 `src/ai/mod.rs`

| Symbol | Role |
|--------|------|
| `AiError` | Provider / config / not-enabled / missing key |
| `OllamaModel`, `OllamaTagsResponse` | `/api/tags` JSON |
| `fetch_models` | Live Ollama list or static cloud lists |
| `fetch_ollama_models` | HTTP GET tags |
| `default_*_models` | Fallback / cloud model name lists |
| `AiProvider` | Holds `OpenAICompatible<DynamicModel>` + config |
| `AiProvider::new` / `chat` / `is_enabled` / `config` | Build client; single-prompt generate |
| `create_provider` | Thin wrapper around `AiProvider::new` |

### 4.4 `src/ai/config.rs`

| Symbol | Role |
|--------|------|
| `AiConfig` | provider, model, base_url, api_key, enabled |
| `default_disabled` | serde default for `enabled` |
| `ProviderType` | Six providers + URL/env/index helpers |
| `AiConfig::resolved_api_key` / `has_usable_api_key` | Config then env; length ≥ 8 |
| `ConfigError` | IO / JSON / not found |

### 4.5 `src/ai/agent.rs`

| Symbol | Role |
|--------|------|
| `AiCommand` | Ask / Do / Plan / Build |
| `from_str` | Parse verb (optional leading `/`) |
| `max_steps` / `system_prompt` | Per-verb loop budget and instructions |
| `run_ai_command` | Full agent run → display lines |
| `parse_tool_call` | Extract tool name + JSON args from model text |

### 4.6 `src/modules/state.rs`

| Symbol | Role |
|--------|------|
| `Entry` / `EntryType` | History rows |
| `SettingsField` / `SettingsPage` | Settings UI model |
| `SettingsState` | Working copy of AI settings + model list |
| `AiLoadingState` | Spinner frames and status line |
| `App` | All interactive state |
| `App::add_entry` / `clear` / `scroll_*` / `visible_*` | History and suggestion paging |
| `word_start_*` / `delete_*` / `yank` | Readline-like editing |
| `current_settings_page` / `settings_push` / `pop` / `move_*` | Settings stack |

### 4.7 `src/modules/render.rs`

| Symbol | Role |
|--------|------|
| `render` | Draw settings or shell |
| `wrap_line` | Width wrap |
| `DisplayRow` | One painted output row |
| `build_display_rows` | History → wrapped rows with styles |
| `render_shell` | Output list, loading bar, input, suggestions |
| `render_settings` | Dispatch page |
| `highlight_style` / `cursor_prefix` | Selected row styling |
| `render_home_page` … `render_enable_page` | One function per settings screen |

### 4.8 `src/modules/keybindings.rs`

| Symbol | Role |
|--------|------|
| `KEY_BINDINGS` | Default combo table |
| `KeyCombo` / `KeyBindings` | Combo description + match |
| `is_backspace` | Terminal Backspace variants |
| `Action` | Semantic key result |
| `get_action` | KeyEvent → Action (emacs/shell extras) |
| `copy_to_clipboard` / `paste_from_clipboard` | arboard (+ Linux primary) |
| `execute_action` | Mutate `App` for editing/clipboard |
| `prev_char_boundary` / `next_char_boundary` | UTF-8 cursor steps |

### 4.9 `src/modules/commands.rs`

| Symbol | Role |
|--------|------|
| `shorten_cwd` | `$HOME` → `~` |
| `normalize_output_lines` | CR/LF and tab columns → TUI lines |
| `is_list_command` / `adjust_list_args` | Force `ls -1` unless user chose a format |
| `execute_command` | Builtins + spawn |

### 4.10 `src/modules/completions.rs`

| Symbol | Role |
|--------|------|
| `PATH_COMMANDS` | Cached PATH executables (by filename) |
| `parse_dir_path` | Split `cd` argument into dir + prefix |
| `update_suggestions` | Fill `App` suggestion list |

### 4.11 `src/modules/config.rs`

Constants only: suggestion/history sizes, scroll steps, prompt `" $ "`, ratatui colors for output/input/suggestions/system.

### 4.12 `src/tools/mod.rs`

| Symbol | Role |
|--------|------|
| `ToolDefinition` | Name, description, JSON Schema parameters |
| `get_tool_definitions` | Catalog injected into the agent prompt |
| `execute_tool` | Name + JSON → tool result JSON |

### 4.13 `src/tools/terminal.rs`

| Symbol | Role |
|--------|------|
| `TerminalError` | IO / not found |
| `cat` / `ls` / `grep` / `grep_dir` | Read tools |
| `FileEntry` | `ls` row |
| `write_file` / `delete_path` / `copy_path` / `move_path` / `mkdir` | Mutating tools |
| `copy_dir_all` | Recursive directory copy |

### 4.14 `src/tools/web_search.rs`

| Symbol | Role |
|--------|------|
| `ToolError` | HTTP / parse |
| `DuckDuckGoResponse` / `RelatedTopic` | API JSON |
| `web_search` | Query Instant Answer API |
| `SearchResult` | title, url, snippet (name collides with `storage::SearchResult`) |

### 4.15 `src/storage/local.rs`

| Symbol | Role |
|--------|------|
| `StorageError` | IO / JSON / path |
| `LocalStorage` | `config_dir()/nsh` JSON files |
| `NshConfig` / `RagConfig` | On-disk schema |
| `load_or_create_config` | Default file if missing |

### 4.16 `src/storage/vector.rs`

| Symbol | Role |
|--------|------|
| `VectorError` | Named as Qdrant but used for file/serde |
| `StoredPoint` | One vector + payload |
| `VectorStore` | Load/save JSON, `add_points`, cosine `search` |
| `SearchResult` | id, score, payload |

### 4.17 `src/rag/mod.rs`

| Symbol | Role |
|--------|------|
| `RagError` | Storage / vector / embedding / init |
| `Document` | id, content, source, metadata |
| `RagEngine` | Embed + VectorStore |
| `new_from_config` / `new` | Construct; derive Ollama embed URL |
| `index_document` / `search` / `retrieve_context` | Write/query/format for agent |
| `embed_text` | POST `/api/embeddings` |
| `RetrievedDocument` | Search hit with score |

---

## 5. Call graph (who uses what)

`lib.rs` exists so the binary stays thin and so modules can be reused/tested as a library.

```mermaid
flowchart LR
  MAIN["main.rs"]
  REND["modules/render.rs"]
  KEYS["modules/keybindings.rs"]
  CMD["modules/commands.rs"]
  FETCH["ai/mod.rs fetch_models"]
  LOC["storage/local.rs"]
  RAG["rag/mod.rs"]
  AG["ai/agent.rs run_ai_command"]
  PROV["ai/mod.rs create_provider"]
  TOOL["tools/mod.rs execute_tool"]
  WEB["tools/web_search.rs"]
  FS["tools/terminal.rs"]
  VEC["storage/vector.rs"]

  MAIN --> REND
  MAIN --> KEYS
  MAIN --> CMD
  MAIN --> FETCH
  MAIN --> LOC
  MAIN --> RAG
  MAIN --> AG
  AG --> PROV
  AG --> RAG
  AG --> TOOL
  RAG --> VEC
  TOOL --> WEB
  TOOL --> FS
```

---

## 6. Key decisions (as implemented)

1. **TUI shell, not a real POSIX shell** — spawn one program + args; no parser for `|`, `>`, quotes.
2. **Human commands vs agent tools split** — agents cannot run arbitrary binaries; they get a fixed tool list.
3. **Text tool protocol** (`TOOL:` / `ARGS:`) instead of provider native tool-calling — works across Ollama and OpenAI-compatible endpoints.
4. **Sync UI + ad-hoc tokio runtimes** — avoids making the whole app async; cost is creating a Runtime per AI/settings/index action.
5. **Mouse capture only in settings** — preserves native copy/select in the output pane.
6. **Local JSON vectors instead of Qdrant** — simpler install; `qdrant-client` leftover.
7. **AI off by default** (`enabled: false`) until the user enables it in settings.
8. **All providers funneled through OpenAI-compatible HTTP** — Anthropic/Gemini use compatibility URLs, not first-party SDKs.

---

## 7. Extension points

| If you want to… | Touch |
|-----------------|--------|
| Add a keybinding | `modules/keybindings.rs` (`KeyBindings` + `get_action` + maybe `main` match) |
| Add a builtin command | `modules/commands.rs` + completions builtins list |
| Add an agent tool | `tools/terminal.rs` or new file, then `get_tool_definitions` + `execute_tool` |
| Add an AI verb | `AiCommand` in `agent.rs` + `main` routing (already uses `from_str`) |
| Change colors / prompt | `modules/config.rs` |
| Persist extra settings | `NshConfig` in `storage/local.rs` + settings pages |
| Real Qdrant | Replace `storage/vector.rs` internals; keep `add_points` / `search` signatures |
| Streaming tokens | `AiProvider::chat` + `main` loading loop (replace block_on generate) |

---

## 8. Detailed Mermaid diagrams

GitHub / most Markdown previews render these as diagrams. Node labels use file and type names from the current source.

### 8.1 System context

Who talks to nsh and over which protocol.

```mermaid
flowchart LR
  U["User at a TTY"]
  NSH["nsh binary"]
  FS["Local filesystem<br/>cwd + ~/.config/nsh"]
  PATH["$PATH programs"]
  CLIP["OS clipboard arboard"]
  OLLAMA["Ollama<br/>localhost:11434"]
  CLOUD["OpenAI / Anthropic / Gemini / OpenRouter<br/>OpenAI-compatible HTTP"]
  DDG["api.duckduckgo.com"]

  U -->|"keyboard mouse paste"| NSH
  NSH -->|"stdout TUI"| U
  NSH --> FS
  NSH -->|"Command::output"| PATH
  NSH --> CLIP
  NSH -->|"chat + /api/tags + /api/embeddings"| OLLAMA
  NSH -->|"chat generate_text"| CLOUD
  NSH -->|"GET Instant Answer JSON"| DDG
```

### 8.2 Crate and module map

Binary crate `nsh` links the library crate `nsh` (`src/lib.rs`).

```mermaid
flowchart TB
  BIN["bin nsh — src/main.rs"]
  LIB["lib nsh — src/lib.rs"]
  BIN --> LIB

  LIB --> AI["ai/"]
  LIB --> MOD["modules/"]
  LIB --> STO["storage/"]
  LIB --> TLS["tools/"]
  LIB --> RAG["rag/"]

  AI --> AI_MOD["mod.rs — AiProvider fetch_models"]
  AI --> AI_CFG["config.rs — ProviderType AiConfig"]
  AI --> AI_AG["agent.rs — AiCommand run_ai_command"]

  MOD --> M_ST["state.rs"]
  MOD --> M_RE["render.rs"]
  MOD --> M_KB["keybindings.rs"]
  MOD --> M_CM["commands.rs"]
  MOD --> M_CO["completions.rs"]
  MOD --> M_CF["config.rs colors"]

  STO --> S_LO["local.rs"]
  STO --> S_VE["vector.rs"]

  TLS --> T_MO["mod.rs dispatcher"]
  TLS --> T_TE["terminal.rs"]
  TLS --> T_WS["web_search.rs"]

  RAG --> R_MO["mod.rs RagEngine"]
```

### 8.3 Main event loop (every tick)

Mouse capture is on **only** while `app.show_settings`. Alt is reconstructed from Esc+key within 25ms.

```mermaid
flowchart TD
  START(["main()"])
  INIT["enable_raw_mode<br/>EnterAlternateScreen<br/>EnableBracketedPaste<br/>App::new + welcome Entry"]
  LOOP{{"running?"}}
  MOUSE{"show_settings XOR mouse_captured?"}
  SYNC["toggle Enable/DisableMouseCapture"]
  DRAW["render terminal, app"]
  POLL{"event::poll 50ms"}
  READ["event::read"]
  KIND{{"Event kind"}}

  START --> INIT --> LOOP
  LOOP -->|no| CLEAN["disable raw, LeaveAlternateScreen, Goodbye"]
  LOOP -->|yes| MOUSE
  MOUSE -->|yes| SYNC --> DRAW
  MOUSE -->|no| DRAW
  DRAW --> POLL
  POLL -->|timeout or err| LOOP
  POLL -->|event| READ --> KIND

  KIND -->|Key Press| KEY
  KIND -->|Paste| PASTE
  KIND -->|Mouse| MOUSEEV
  KIND -->|other| LOOP

  subgraph KEY["Key"]
    SET{"show_settings?"}
    SET -->|yes| HSET["handle_settings_input"]
    SET -->|no| ALT["resolve_key_with_alt_meta"]
    ALT --> ACT["get_action KeyCode modifiers"]
    ACT --> DISP{"Action"}
  end

  HSET --> LOOP

  DISP -->|OpenSettings| OS["load_settings_state<br/>fetch_models on tokio rt<br/>show_settings true"]
  DISP -->|Interrupt| INT["add ^C Entry, clear input"]
  DISP -->|Eof and empty| STOP["running = false"]
  DISP -->|Execute| EX["see 8.4 command router"]
  DISP -->|Complete / History / Page| HIST["suggestion or history logic in main"]
  DISP -->|Cancel| CAN["hide suggestions"]
  DISP -->|other| EA["execute_action App"]

  OS --> LOOP
  INT --> LOOP
  STOP --> LOOP
  EX --> LOOP
  HIST --> LOOP
  CAN --> LOOP
  EA --> LOOP

  PASTE{"show_settings?"}
  PASTE -->|yes| SP["settings_apply_paste"]
  PASTE -->|no| IP["insert into current_input at cursor"]
  SP --> LOOP
  IP --> LOOP

  MOUSEEV{"settings + left click?"}
  MOUSEEV -->|yes| CLK["map row to item, settings_handle_enter"]
  MOUSEEV -->|middle in settings| MP["paste_from_clipboard"]
  MOUSEEV -->|wheel| SCR["scroll suggestions or history"]
  CLK --> LOOP
  MP --> LOOP
  SCR --> LOOP
```

### 8.4 Enter key — command router

AI verbs are intercepted **before** `execute_command`. Sentinels `__SETTINGS__` and `__CLEAR__` are the only control codes returned by the shell path.

```mermaid
flowchart TD
  ENTER["Action::Execute"]
  IN["clone current_input"]
  EXIT{"input is exit or quit?"}
  EMPTY{"empty?"}
  ADD["add_entry Command with cwd"]
  FIRST["first token, strip leading /"]

  ENTER --> IN --> EXIT
  EXIT -->|yes| DIE["running = false"]
  EXIT -->|no| EMPTY
  EMPTY -->|yes| RESET["clear input cursor history suggestions"]
  EMPTY -->|no| ADD --> FIRST

  FIRST --> IDX{"cmd is index?"}
  IDX -->|yes| INDEX["LocalStorage + RagEngine<br/>index_directory_simple path or .<br/>max 50 files, one directory level"]
  INDEX --> OUT1["add_entry Output"]

  IDX -->|no| AIC{"AiCommand::from_str?"}
  AIC -->|ask do plan build| AI["set AiLoadingState<br/>render<br/>thread + mpsc<br/>spin until result"]
  AI --> OUT2["add_entry Output or System"]

  AIC -->|no| EC["execute_command input"]
  EC --> SENT{{"output contains"}}
  SENT -->|"__SETTINGS__"| SET["open settings + fetch_models"]
  SENT -->|"__CLEAR__"| CLR["app.clear"]
  SENT -->|other non-empty| OUT3["add_entry Output"]
  SENT -->|empty| RESET

  OUT1 --> RESET
  OUT2 --> RESET
  SET --> RESET
  CLR --> RESET
  OUT3 --> RESET
```

### 8.5 Human shell vs agent tools (two paths)

These are **different implementations**. Typing `ls` runs `/usr/bin/ls`. The model calling `ls` runs `tools::terminal::ls`.

```mermaid
flowchart LR
  subgraph human["Path A — user typed a line"]
    U["current_input"]
    EC["execute_command"]
    BI{"builtin?"}
    CD["cd set_current_dir"]
    CL["return __CLEAR__"]
    ST["return __SETTINGS__"]
    HP["help text"]
    SP["Command::new program .args .output"]
    NORM["normalize_output_lines<br/>ls gets -1 unless format flag"]
    U --> EC --> BI
    BI -->|cd| CD
    BI -->|clear| CL
    BI -->|settings| ST
    BI -->|help| HP
    BI -->|else| SP --> NORM
  end

  subgraph agent["Path B — model TOOL line"]
    P["parse_tool_call"]
    ET["execute_tool"]
    M{"name"}
    P --> ET --> M
    M -->|web_search| DDG["HTTP DuckDuckGo"]
    M -->|cat ls grep| RD["in-process read"]
    M -->|write_file delete_path<br/>copy_path move_path mkdir| WR["in-process mutate cwd"]
  end

  NOTE["No pipes, quotes, glob, or job control on Path A.<br/>No arbitrary exec on Path B."]
```

### 8.6 `execute_command` builtins and spawn

```mermaid
flowchart TD
  IN["trimmed input"]
  SPLIT["split_whitespace<br/>program + raw_args"]
  ADJ["adjust_list_args<br/>if ls/dir/vdir/lsd/exa/eza/tree<br/>and no format flag: prepend -1"]
  M{"program"}

  IN --> SPLIT --> ADJ --> M
  M -->|exit quit| E1["empty vec — main already handled"]
  M -->|cd| CD["HOME or arg; set_current_dir"]
  M -->|clear| C2["vec __CLEAR__"]
  M -->|help /help| H["static help lines"]
  M -->|settings /settings| S["vec __SETTINGS__"]
  M -->|ask do plan build index<br/>with or without slash| STUB["routing stub string<br/>not used if main intercepted"]
  M -->|_| EXT["Command::new output"]
  EXT -->|ok| SO["stdout then stderr via normalize_output_lines"]
  EXT -->|err| NF["program: command not found"]
  SO -->|empty and fail| ST["exited with status N"]
```

### 8.7 Completions

```mermaid
flowchart TD
  US["update_suggestions App"]
  EMP{"current_input empty?"}
  CD{"starts with cd space?"}
  EMP -->|yes| HIDE["clear suggestions, hide popup"]
  EMP -->|no| CD
  CD -->|yes| DIR["parse_dir_path<br/>read_dir base, dirs whose name matches prefix<br/>sort by display"]
  CD -->|no| BUILT["prefix match ask /ask do /do<br/>plan /plan build /build<br/>index /index /settings /help"]
  BUILT --> PATH["PATH_COMMANDS prefix match"]
  PATH --> HIST["history Command entries prefix match<br/>stop around 20"]
  DIR --> SHOW["show_suggestions if non-empty<br/>selected = 0"]
  HIST --> SHOW
```

### 8.8 Agent ReAct loop (`run_ai_command`)

```mermaid
flowchart TD
  START["run_ai_command cmd query ai_config storage"]
  EN{"ai_config.enabled?"}
  EN -->|no| ERR1["Err NotEnabled"]
  EN -->|yes| PROV["create_provider"]
  PROV -->|MissingApiKey or build fail| ERR2["Err"]
  PROV -->|ok| RAGTRY["RagEngine::new_from_config .ok"]
  RAGTRY --> CTX["retrieve_context query 3 or empty"]
  CTX --> HIST["history vec:<br/>System prompt<br/>Tools JSON<br/>optional RAG block<br/>cwd<br/>User query"]
  HIST --> STEP["step = 0"]
  STEP --> LOOP{"step less than max_steps?"}
  LOOP -->|no| FALL["fallback: agent finished without a clear final message"]
  LOOP -->|yes| CHAT["provider.chat history joined with blank lines"]
  CHAT --> PARSE{"parse_tool_call"}
  PARSE -->|Some name args| EXEC["execute_tool"]
  EXEC --> OBS["push Assistant text + Observation JSON"]
  OBS --> INC["step++"] --> LOOP
  PARSE -->|None| ANS["final_answer = response"]
  ANS --> LINES["Ok split lines"]
  FALL --> LINES
```

Per-verb budgets and prompts:

```mermaid
flowchart LR
  subgraph verbs["AiCommand"]
    ASK["Ask — 1 step<br/>answer; at most one tool"]
    DO["Do — 3 steps<br/>small FS mutate"]
    PLAN["Plan — 2 steps<br/>read-only preferred"]
    BUILD["Build — 6 steps<br/>explore then write_file"]
  end
```

### 8.9 Tool-call parser

```mermaid
flowchart TD
  T["model text trim"]
  A{"contains TOOL:?"}
  A -->|yes| NAME["tool = first line after TOOL:"]
  NAME --> B{"contains ARGS:?"}
  B -->|yes| J1["parse first line as JSON"]
  J1 -->|fail| J2["parse remaining as JSON"]
  J2 -->|ok| HIT["Some name, Value"]
  J1 -->|ok| HIT
  A -->|no| C{"find first { to last }"}
  B -->|no| C
  C -->|json has name| D["name + arguments"]
  C -->|json has tool| E["tool + args"]
  C -->|else| NONE["None — treat as final answer"]
  D --> HIT
  E --> HIT
```

### 8.10 Tool dispatcher

```mermaid
flowchart TB
  ET["execute_tool name args"]
  ET --> SW{"name"}

  SW -->|web_search| WS["query required<br/>web_search await"]
  SW -->|cat| CAT["path required — read_to_string"]
  SW -->|ls| LS["optional path default .<br/>dirs first then name"]
  SW -->|grep| GR["pattern required<br/>file: line matches<br/>dir: depth 3, ext filter"]
  SW -->|write_file| WF["path + content<br/>create_dir_all parent"]
  SW -->|delete_path| DEL["file unlink or remove_dir_all"]
  SW -->|copy_path| CP["file copy or copy_dir_all"]
  SW -->|move_path| MV["rename; create parent"]
  SW -->|mkdir| MD["create_dir_all"]
  SW -->|_| UNK["Err Unknown tool"]

  WS --> JSON["Ok serde_json Value"]
  CAT --> JSON
  LS --> JSON
  GR --> JSON
  WF --> JSON
  DEL --> JSON
  CP --> JSON
  MV --> JSON
  MD --> JSON
```

### 8.11 RAG index vs retrieve

`index` is a user command. Retrieve happens inside every AI verb (best-effort; failure is ignored).

```mermaid
sequenceDiagram
  participant User
  participant Main
  participant Walk as index_directory_simple
  participant Eng as RagEngine
  participant Emb as POST /api/embeddings
  participant VS as VectorStore JSON

  Note over User,VS: Index path
  User->>Main: index ./src or /index path
  Main->>Eng: new_from_config
  Main->>Walk: walk files max 50
  Walk->>Walk: ext rs toml md txt json yaml yml py js ts go sh
  loop each file
    Walk->>Eng: index_document Document
    Eng->>Emb: embed content
    Emb-->>Eng: Vec f32
    Eng->>VS: add_points + save rag/nsh_rag.json
  end
  Eng-->>User: Indexed N documents…

  Note over User,VS: Retrieve path inside agent
  Main->>Eng: retrieve_context query k=3
  Eng->>Emb: embed query
  Emb-->>Eng: query vector
  Eng->>VS: cosine search limit
  VS-->>Eng: payload content source score
  Eng-->>Main: compact context string or empty
```

Embed URL: strip trailing `/v1` from Ollama / OpenAICompatible `base_url`; otherwise `http://localhost:11434`. Model: `ai.model` else `nomic-embed-text`. **`rag.embed_model` in config is not read.**

Vector search:

```mermaid
flowchart LR
  Q["query Vec f32"]
  P["points in memory"]
  C["cosine a·b / |a||b|"]
  S["sort score descending"]
  T["take limit"]
  R["SearchResult id=index score payload"]
  Q --> C
  P --> C --> S --> T --> R
```

### 8.12 Settings navigation stack

`settings_nav: Vec<SettingsPage>`. Empty stack while `show_settings` means Home.

```mermaid
stateDiagram-v2
  [*] --> Shell
  Shell --> Home: Ctrl+, / settings / __SETTINGS__
  Home --> Provider: Enter on Provider
  Home --> Model: Enter on Model if list non-empty
  Home --> BaseUrl: Enter on Base URL
  Home --> ApiKey: Enter on API Key
  Home --> Enable: Enter on Enable
  Home --> Shell: Save writes config.json
  Home --> Shell: Cancel or Esc on Home

  Provider --> Home: Enter sets provider, default URL, fetch_models, first model
  Model --> Home: Enter sets model
  BaseUrl --> Home: Enter or Esc
  ApiKey --> Home: Enter copies api_key_original
  Enable --> Home: Enter cursor 0 = enabled

  Provider --> Home: Esc pop
  Model --> Home: Esc pop
  BaseUrl --> Home: Esc pop
  ApiKey --> Home: Esc pop
  Enable --> Home: Esc pop
```

Home cursor indices:

```mermaid
flowchart LR
  F0["0 Provider"] --> F1["1 Model"] --> F2["2 Base URL"] --> F3["3 API Key"] --> F4["4 Enable"] --> F5["5 Save"] --> F6["6 Cancel"]
```

### 8.13 `App` and related types

```mermaid
classDiagram
  class App {
    +Vec~Entry~ entries
    +String current_input
    +usize cursor_position
    +usize scroll_offset
    +usize total_lines
    +Vec suggestions
    +bool show_suggestions
    +usize selected_suggestion
    +String saved_input
    +Option~usize~ history_index
    +Vec~String~ kill_ring
    +bool show_settings
    +SettingsState settings_state
    +usize settings_cursor
    +Vec~SettingsPage~ settings_nav
    +Option~AiLoadingState~ ai_loading
    +add_entry()
    +clear()
    +update_suggestions()
    +word_start_backward()
    +delete_word_before()
    +yank()
    +settings_push()
    +settings_pop()
  }

  class Entry {
    +EntryType entry_type
    +Vec~String~ content
    +String cwd
  }

  class EntryType {
    <<enumeration>>
    Command
    Output
    System
  }

  class SettingsState {
    +ProviderType provider
    +String model
    +String base_url
    +String api_key
    +String api_key_original
    +bool enabled
    +Vec~String~ available_models
  }

  class SettingsPage {
    <<enumeration>>
    Home Provider Model BaseUrl ApiKey Enable
  }

  class SettingsField {
    <<enumeration>>
    Provider Model BaseUrl ApiKey Enable Save Cancel
  }

  class AiLoadingState {
    +String verb
    +String provider
    +String model
    +usize frame
    +spinner_glyph()
    +status_line()
  }

  class AiConfig {
    +ProviderType provider
    +String model
    +String base_url
    +Option~String~ api_key
    +bool enabled
    +resolved_api_key()
    +has_usable_api_key()
  }

  class NshConfig {
    +AiConfig ai
    +RagConfig rag
  }

  App "1" --> "*" Entry
  Entry --> EntryType
  App --> SettingsState
  App --> SettingsPage
  App --> AiLoadingState
  SettingsState --> ProviderType
  NshConfig --> AiConfig
```

### 8.14 Key → Action → handler

Some actions mutate `App` in `execute_action`. Others **must** stay in `main` because they need cwd, spawn, or settings I/O.

```mermaid
flowchart LR
  subgraph keys["crossterm KeyEvent"]
    K["KeyCode + KeyModifiers"]
  end
  K --> G["get_action"]
  G --> A["Action enum"]

  subgraph main_only["Handled in main.rs"]
    A1["OpenSettings Interrupt Eof Execute"]
    A2["Complete HistoryUp HistoryDown"]
    A3["SuggestionPageUp PageDown Cancel"]
  end

  subgraph exec["execute_action"]
    E1["Move* Delete* Yank Copy Paste InsertChar"]
  end

  A --> A1
  A --> A2
  A --> A3
  A --> E1
```

### 8.15 Render pipeline

```mermaid
flowchart TD
  R["render terminal, app"]
  R --> Q{"show_settings?"}
  Q -->|yes| RS["render_settings"]
  RS --> P{"current_settings_page"}
  P -->|Home| H["render_home_page"]
  P -->|Provider| PR["render_provider_page"]
  P -->|Model| MO["render_model_page"]
  P -->|BaseUrl| BU["render_baseurl_page"]
  P -->|ApiKey| AK["render_apikey_page masked"]
  P -->|Enable| EN["render_enable_page"]

  Q -->|no| SH["render_shell"]
  SH --> LAY{"ai_loading?"}
  LAY -->|yes| L3["chunks: Min output + 1 loading + 3 input"]
  LAY -->|no| L2["chunks: Min output + 3 input"]
  L3 --> WRAP
  L2 --> WRAP["build_display_rows wrap_width"]
  WRAP --> W["wrap_line whitespace then hard wrap"]
  W --> STYLE["Command: green cwd + gray rest<br/>Output white<br/>System dark gray"]
  STYLE --> LIST["List widget scrolled"]
  LIST --> LOAD["optional yellow status bar"]
  LOAD --> INP["input Paragraph + real cursor<br/>or hidden cursor + wait hint"]
  INP --> SUG["suggestion overlay 40 cols above input"]
```

### 8.16 Provider and API key resolution

```mermaid
flowchart TD
  NEW["AiProvider::new AiConfig"]
  NEED{"provider.requires_api_key?<br/>false only for Ollama"}
  NEED -->|no| BUILD
  NEED -->|yes| USE{"has_usable_api_key?<br/>resolved length at least 8"}
  USE -->|no| MISS["Err MissingApiKey<br/>lists env vars"]
  USE -->|yes| BUILD["OpenAICompatible builder<br/>model_name, trim base_url, api_key"]

  subgraph resolve["AiConfig::resolved_api_key"]
    C{"config.api_key trim non-empty?"}
    C -->|yes| K1["use config key"]
    C -->|no| ENV["first non-empty among provider env vars"]
    ENV --> K2["OPENAI_API_KEY / ANTHROPIC_API_KEY<br/>GEMINI_API_KEY GOOGLE_API_KEY<br/>OPENROUTER_API_KEY / NSH_API_KEY"]
  end

  BUILD --> CHAT["LanguageModelRequest generate_text"]
```

Default URLs:

```mermaid
flowchart LR
  O["Ollama — http://localhost:11434"]
  OA["OpenAI — https://api.openai.com/v1"]
  AN["Anthropic — https://api.anthropic.com"]
  GE["Gemini — generativelanguage.googleapis.com/v1beta/openai/"]
  OR["OpenRouter — https://openrouter.ai/api/v1"]
  OC["OpenAI Compatible — http://localhost:11434/v1"]
```

`fetch_models`: Ollama GET `{base}/api/tags`; everyone else uses a hardcoded name list in `ai/mod.rs`.

### 8.17 Threading and runtimes

The UI thread never `.await`. Tokio exists only inside `block_on` islands.

```mermaid
flowchart TB
  subgraph ui["UI thread — sync"]
    EV["crossterm event loop"]
    RD["ratatui draw"]
    SP["spinner frame++ every ~80ms during AI"]
  end

  subgraph islands["tokio Runtime::new block_on — short lived"]
    FM["fetch_models when opening settings or changing provider"]
    IX["index_directory_simple on UI thread"]
  end

  subgraph bg["spawned OS thread per AI verb"]
    RT["its own tokio Runtime"]
    AG["run_ai_command await chat embed tools"]
    TX["mpsc::Sender Result"]
  end

  EV --> FM
  EV --> IX
  EV -->|"ask do plan build"| bg
  bg -->|"mpsc recv"| SP
  SP --> RD
```

### 8.18 On-disk layout and types

```mermaid
flowchart TB
  subgraph disk["~/.config/nsh/"]
    CJ["config.json"]
    RJ["rag/nsh_rag.json"]
  end

  CJ --> NC["NshConfig"]
  NC --> AI["AiConfig"]
  NC --> RC["RagConfig embed_model unused at runtime"]
  RJ --> PTS["Vec StoredPoint"]
  PTS --> SP["vector Vec f32<br/>payload id content source metadata"]

  LS["LocalStorage base_path = dirs::config_dir / nsh"]
  LS --> CJ
  VS["VectorStore persist_path"]
  VS --> RJ
```

### 8.19 Startup to first prompt

```mermaid
sequenceDiagram
  participant OS
  participant Main
  participant CT as crossterm
  participant App
  participant Rend as render

  OS->>Main: exec nsh
  Main->>CT: enable_raw_mode
  Main->>CT: EnterAlternateScreen EnableBracketedPaste
  Main->>App: App::new
  Main->>App: add_entry System welcome
  loop until exit/quit or Ctrl+D on empty
    Main->>Rend: draw shell or settings
    Main->>CT: poll 50ms
    CT-->>Main: Key / Paste / Mouse / timeout
    Main->>App: mutate
  end
  Main->>CT: DisableMouseCapture DisableBracketedPaste LeaveAlternateScreen
  Main->>OS: println Goodbye
```

### 8.20 Error paths the user can see

```mermaid
flowchart TD
  subgraph shell["Shell"]
    NF["program: command not found"]
    CD["cd: path: io error"]
    ST["program: exited with status N"]
  end

  subgraph ai["AI verb"]
    DIS["AI not enabled"]
    KEY["No valid API key for Provider…"]
    RT["AI error: runtime: …"]
    PR["AI error: Provider error: …"]
    EMPTY["System: verb finished empty response"]
    DISC["AI task ended unexpectedly"]
    FALL["The agent finished without a clear final message"]
  end

  subgraph rag["index"]
    INIT["RAG init failed: …"]
    ZERO["Indexed 0 documents if path missing"]
  end
```

These strings are added as `EntryType::Output` or `EntryType::System` and painted by `render_shell`.
