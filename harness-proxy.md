Yes. I’d add this to `nsh`, but I would **not** implement it as another LLM provider. The right abstraction is a **Harness/Agent Proxy layer** sitting beside your existing AI provider layer.

I checked the current `nsh` architecture: it already has a provider-agnostic AI layer and external-process spawning in `modules/commands.rs`, so you have a good foundation for this. ([GitHub][1])

Also, the ecosystem is moving toward exactly this separation: Zed treats Claude Code/Codex/OpenCode etc. as external agents, while T3 Code launches provider CLIs as subprocesses. ([Zed][2])

### What I would build

Think of `nsh` as becoming:

```text
                         ┌─────────────────────┐
                         │        nsh          │
                         │  Agentic Shell/TUI  │
                         └──────────┬──────────┘
                                    │
                  ┌─────────────────┴─────────────────┐
                  │                                   │
            AI Provider                         Agent Harness
                  │                                   │
       ┌──────────┼──────────┐             ┌──────────┼──────────┐
       │          │          │             │          │          │
    OpenAI     Anthropic   Ollama        Claude     Codex     OpenCode
                                      Code CLI       CLI
                                                    │
                                      ┌─────────────┼─────────────┐
                                      │             │             │
                                  Grok Build   Antigravity     T3/Zed
```

The important distinction is:

**Provider** = model/API.

**Harness** = coding agent that owns its own context, tools, permissions, authentication, filesystem interaction and agent loop.

That distinction matters enormously. A Claude Code session is not equivalent to calling the Anthropic API.

Current projects are already converging on this model. For example, T3 Code runs Claude/Codex/OpenCode as provider backends, while Zed integrates coding agents through an external-agent/ACP model. ([oh.mifune.dev][3])

## The feature I'd propose for `nsh`

I'd call it **Nsh Harness** or simply **Agent Harness**.

Users should be able to do:

```bash
nsh agent claude
nsh agent codex
nsh agent opencode
nsh agent grok
nsh agent antigravity
```

or, more naturally inside your shell:

```bash
/agent claude
/agent codex
/agent opencode
/agent grok
/agent antigravity
```

Then:

```text
nsh:~/project $ /agent
```

opens:

```text
╭──────────────────────────────────────╮
│           Select AI Agent             │
│                                      │
│  ● Claude Code                       │
│    OpenAI Codex                      │
│    OpenCode                          │
│    Grok Build                        │
│    Antigravity                       │
│                                      │
│  Enter → launch                      │
│  Esc   → cancel                      │
╰──────────────────────────────────────╯
```

The agent should launch **inside the current directory**, inheriting the user's environment and terminal rather than trying to reimplement each agent's UI.

That's the first version.

---

# The proxy part

If by "Harness proxy like Zed/T3Code" you mean a layer that sits **between nsh and the underlying agent**, then I'd make this explicit:

```text
nsh
 │
 │ AgentRequest
 ▼
HarnessManager
 │
 ├── ClaudeAdapter
 ├── CodexAdapter
 ├── OpenCodeAdapter
 ├── GrokAdapter
 └── AntigravityAdapter
       │
       ▼
   subprocess
       │
       ▼
   actual CLI
```

Each adapter should know:

```rust
pub trait AgentHarness {
    fn name(&self) -> &str;
    fn command(&self) -> &str;
    fn is_installed(&self) -> bool;
    fn launch(&self, context: AgentContext) -> Result<AgentSession>;
}
```

Then:

```rust
pub struct AgentContext {
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub terminal: TerminalContext,
}
```

And:

```rust
pub enum Harness {
    ClaudeCode,
    Codex,
    OpenCode,
    GrokBuild,
    Antigravity,
}
```

This gives you a stable internal API even if the individual CLIs change.

---

# Don't make nsh parse every agent's protocol initially

This is where I'd push back against trying to copy Zed's entire architecture immediately.

For V1, **do not attempt to normalize every agent's streaming protocol, tool calls, permissions and conversation state**.

Just create a process abstraction:

```text
nsh
 ↓
HarnessManager
 ↓
PTY
 ↓
claude / codex / opencode / grok / agy
```

A PTY is important.

Don't simply do:

```rust
Command::new("claude").spawn()
```

if you want a serious interactive integration.

Instead:

```text
nsh
  │
  └── PTY
       │
       ├── stdin
       ├── stdout
       └── stderr
            │
            ▼
       Claude Code
```

That preserves the interactive terminal experience.

This is also much closer to how a shell should integrate with existing agent CLIs.

---

# Then V2 becomes the actual Harness Proxy

Once V1 works, introduce:

```text
HarnessSession
```

with:

```rust
pub struct HarnessSession {
    pub id: String,
    pub harness: Harness,
    pub pid: u32,
    pub cwd: PathBuf,
    pub started_at: DateTime<Utc>,
    pub status: SessionStatus,
}
```

Then:

```bash
nsh agent list
```

could show:

```text
AGENT             STATUS       PID       DIRECTORY

Claude Code       running      18421     ~/projects/nsh
Codex             idle         19231     ~/projects/api
OpenCode          running      20312     ~/projects/web
```

And eventually:

```bash
nsh agent attach <id>
nsh agent stop <id>
nsh agent restart <id>
```

Now `nsh` starts becoming an actual **terminal-native agent manager**, rather than just an AI shell.

---

# Configuration

I'd extend your existing:

```text
~/.config/nsh/config.json
```

with:

```json
{
  "ai": {
    "provider": "Ollama",
    "model": "llama3.2:latest",
    "base_url": "http://localhost:11434",
    "api_key": null,
    "enabled": false
  },
  "agents": {
    "default": "claude",
    "auto_detect": true,
    "agents": {
      "claude": {
        "command": "claude",
        "enabled": true
      },
      "codex": {
        "command": "codex",
        "enabled": true
      },
      "opencode": {
        "command": "opencode",
        "enabled": true
      },
      "grok": {
        "command": "grok",
        "enabled": true
      },
      "antigravity": {
        "command": "agy",
        "enabled": true
      }
    }
  }
}
```

But I'd actually make the configuration even more flexible:

```json
{
  "agents": {
    "claude": {
      "command": "claude",
      "args": []
    },
    "codex": {
      "command": "codex",
      "args": []
    }
  }
}
```

That allows users to add arbitrary agents later without requiring a new Rust enum.

---

# Auto-detection

This would be a killer UX feature.

When `nsh` starts:

```text
Detecting agents...

✓ Claude Code
✓ Codex
✓ OpenCode
✗ Grok Build
✓ Antigravity
```

Then:

```bash
nsh agent
```

only shows installed agents.

You can detect using:

```bash
which claude
which codex
which opencode
which grok
which agy
```

but implement it using Rust's PATH lookup rather than spawning shells.

---

# The really interesting part: proxying model requests

There are actually **two different proxy concepts**, and you should decide which one you're building.

### 1. Agent proxy

```text
nsh → Claude Code
```

nsh manages the CLI.

This is straightforward and should be V1.

### 2. Model/API proxy

```text
Claude Code
      ↓
   nsh proxy
      ↓
 OpenAI / Anthropic / OpenRouter / local model
```

This is much more powerful.

For example:

```text
                 ┌───────────────┐
Claude Code ────►│               │
Codex CLI ──────►│  nsh proxy    │────► OpenRouter
OpenCode ───────►│               │────► Ollama
Grok Build ─────►│               │────► OpenAI
Antigravity ────►│               │────► Anthropic
                 └───────────────┘
```

That would let the user say:

```bash
nsh proxy start
```

and get:

```text
nsh proxy
  Status: running
  Address: http://127.0.0.1:PORT
```

Then compatible agents can point to it.

However, **don't assume every proprietary CLI will happily accept an arbitrary OpenAI-compatible endpoint**. Their authentication, API formats, streaming behavior and tool semantics differ. Claude Code in particular has had several proxy/environment compatibility issues, so this layer needs careful per-agent testing rather than a blanket `HTTP_PROXY` solution. ([GitHub][4])

---

# My recommended architecture

I'd structure your Rust project like this:

```text
src/
├── main.rs
├── lib.rs
│
├── ai/
│   ├── mod.rs
│   ├── config.rs
│   ├── providers.rs
│   └── streaming.rs
│
├── agents/
│   ├── mod.rs
│   ├── manager.rs
│   ├── registry.rs
│   ├── session.rs
│   ├── pty.rs
│   │
│   ├── claude.rs
│   ├── codex.rs
│   ├── opencode.rs
│   ├── grok.rs
│   └── antigravity.rs
│
├── proxy/
│   ├── mod.rs
│   ├── server.rs
│   ├── router.rs
│   ├── auth.rs
│   └── providers.rs
│
├── modules/
│   ├── commands.rs
│   ├── completions.rs
│   ├── render.rs
│   ├── state.rs
│   └── ...
│
└── storage/
    └── ...
```

The important separation is:

```text
AI
 └── talks to models

Agents
 └── talks to coding-agent CLIs

Proxy
 └── translates/routes network/model requests

Shell
 └── orchestrates everything
```

Your current repository already has the AI/provider layer and external process spawning, so this fits naturally rather than requiring a rewrite. ([GitHub][1])

---

# Agent capability matrix

I'd also introduce capabilities rather than assuming every agent behaves identically:

```rust
bitflags! {
    pub struct AgentCapabilities: u32 {
        const INTERACTIVE = 1 << 0;
        const HEADLESS    = 1 << 1;
        const PTY         = 1 << 2;
        const ACP         = 1 << 3;
        const MCP         = 1 << 4;
        const MODEL_SELECT = 1 << 5;
        const RESUME       = 1 << 6;
    }
}
```

For example:

```text
                PTY   Headless   MCP   ACP
Claude Code     ✓       ✓        ✓     ?
Codex           ✓       ✓        ✓     ?
OpenCode        ✓       ✓        ✓     ?
Grok Build      ✓       ✓        ✓     ✓
Antigravity     ✓       ✓        ✓     ✓
```

The exact capability matrix should be discovered from each CLI's current documentation rather than hardcoded based on assumptions.

---

# Why this is better than copying T3 Code

T3 Code's model is useful because it launches provider CLIs as subprocesses. ([oh.mifune.dev][3])

But `nsh` has a different opportunity.

T3:

```text
Web/Desktop UI
      ↓
T3 backend
      ↓
Agent CLI
```

Your:

```text
Terminal
   ↓
nsh
   ↓
Agent Harness
   ↓
Agent CLI
```

So `nsh` could become the **Linux-native universal agent shell**.

And there is already precedent for this direction: other projects are building unified harness layers around Claude Code, Codex, OpenCode and other agents rather than treating each as a completely separate application. ([GitHub][5])

---

# Proposal you can put in the repository

I would make this a GitHub issue/RFC rather than just opening a vague "support Claude/Codex" issue.

# RFC: Universal AI Agent Harness & Proxy for nsh

## Summary

Add a first-class **Agent Harness** subsystem to `nsh` that allows users to discover, launch, manage, and eventually proxy multiple AI coding agents from a single terminal-native interface.

The initial target agents are:

* Claude Code
* OpenAI Codex CLI
* OpenCode
* Grok Build
* Antigravity CLI

The goal is not to reimplement these agents inside `nsh`. Instead, `nsh` should provide a unified orchestration layer around the existing CLIs while preserving their native capabilities and authentication flows.

## Motivation

`nsh` already provides an agentic shell with an abstraction for AI providers and external process spawning. However, model providers and coding-agent harnesses are fundamentally different concepts.

An LLM provider exposes a model/API.

A coding-agent harness provides an autonomous development environment with its own tools, context, permissions, authentication, terminal interaction, MCP support, session state, and agent loop.

Claude Code, Codex, OpenCode, Grok Build, and Antigravity should therefore not be represented merely as additional `AiProvider` implementations.

Instead, `nsh` should introduce a separate `agents` subsystem.

## Proposed Architecture

```text
                         nsh
                          │
             ┌────────────┴────────────┐
             │                         │
        AI Providers              Agent Harness
             │                         │
      ┌──────┼──────┐        ┌─────────┼─────────┐
      │      │      │        │         │         │
   OpenAI  Claude  Ollama  Claude    Codex    OpenCode
                           Code       CLI
                                      │
                              ┌───────┼────────┐
                              │       │        │
                            Grok  Antigravity  ...
```

The agent subsystem should expose a common interface:

```rust
pub trait AgentHarness {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn command(&self) -> &str;
    fn detect(&self) -> bool;
    fn launch(&self, context: AgentContext) -> Result<AgentSession>;
}
```

The initial implementation should use PTY-backed subprocesses so interactive agents retain their native terminal UI and behavior.

## User Experience

Users should be able to run:

```bash
nsh agent
```

and receive a TUI picker:

```text
╭──────────────────────────────────────╮
│           Select AI Agent             │
│                                      │
│  ● Claude Code                       │
│    OpenAI Codex                      │
│    OpenCode                          │
│    Grok Build                        │
│    Antigravity                       │
│                                      │
│  Enter → launch                      │
│  Esc   → cancel                      │
╰──────────────────────────────────────╯
```

Direct launching should also be supported:

```bash
nsh agent claude
nsh agent codex
nsh agent opencode
nsh agent grok
nsh agent antigravity
```

Within the interactive shell, equivalent slash commands could be provided:

```text
/agent
/agent claude
/agent codex
```

The selected agent should start in the current working directory.

## Agent Discovery

`nsh` should automatically detect installed agents through the user's PATH.

Example:

```text
Agent detection

✓ Claude Code
✓ OpenAI Codex
✓ OpenCode
✗ Grok Build
✓ Antigravity
```

Only detected/enabled agents should appear in the default selector.

Users should also be able to configure custom agent commands.

## Configuration

Extend the existing `nsh` configuration with an `agents` section:

```json
{
  "agents": {
    "default": "claude",
    "auto_detect": true,
    "agents": {
      "claude": {
        "command": "claude",
        "args": []
      },
      "codex": {
        "command": "codex",
        "args": []
      },
      "opencode": {
        "command": "opencode",
        "args": []
      },
      "grok": {
        "command": "grok",
        "args": []
      },
      "antigravity": {
        "command": "agy",
        "args": []
      }
    }
  }
}
```

The registry should support arbitrary custom agents so adding a new CLI does not require changing core shell architecture.

## Session Management

The agent layer should eventually expose:

```bash
nsh agent list
nsh agent attach <id>
nsh agent stop <id>
nsh agent restart <id>
```

Example:

```text
ID       AGENT          STATUS     PID      DIRECTORY

a81f     Claude Code    running    18421    ~/projects/nsh
b72c     Codex          idle       19231    ~/projects/api
c91d     OpenCode       running    20312    ~/projects/web
```

This should be implemented after the basic launch flow is stable.

## Harness Proxy

A second phase should introduce an optional local proxy layer.

```text
Claude Code ──┐
Codex CLI ────┤
OpenCode ─────┤
Grok Build ───┤──► nsh proxy ───► model providers
Antigravity ──┘
```

The proxy should provide:

* local loopback endpoint
* provider routing
* configurable model routing
* request/response logging with secrets redacted
* optional model aliases
* provider failover
* configurable authentication
* health checks

Example:

```bash
nsh proxy start
nsh proxy status
nsh proxy stop
```

The proxy should remain independent from the agent process manager.

Not every agent should be assumed to support an arbitrary OpenAI-compatible endpoint. Provider-specific adapters may be required.

## PTY Architecture

Interactive agents should be launched through a PTY rather than ordinary stdout/stderr pipes.

```text
nsh
 │
 ▼
PTY
 │
 ├── stdin
 ├── stdout
 └── stderr
      │
      ▼
claude / codex / opencode / grok / agy
```

This preserves:

* terminal colors
* cursor control
* interactive prompts
* authentication flows
* agent-native TUIs
* terminal resize events
* Ctrl-C/Ctrl-D behavior

## Capability Model

Agents should advertise capabilities rather than being assumed to have identical features.

Potential capabilities include:

```rust
pub struct AgentCapabilities {
    pub interactive: bool,
    pub headless: bool,
    pub pty: bool,
    pub mcp: bool,
    pub acp: bool,
    pub model_selection: bool,
    pub resume: bool,
}
```

This allows `nsh` to expose functionality only when supported by the selected agent.

## Implementation Phases

### Phase 1 — Agent Registry

Implement:

* `AgentHarness` trait
* agent registry
* PATH detection
* configuration
* Claude Code support
* Codex support
* OpenCode support

### Phase 2 — PTY Runner

Implement:

* PTY process management
* terminal resize
* signal forwarding
* process lifecycle
* current-directory inheritance
* environment inheritance

### Phase 3 — TUI Integration

Add:

```bash
nsh agent
```

with agent discovery and selection.

Add:

```text
/agent
```

to the interactive shell.

### Phase 4 — Additional Agents

Add adapters/configuration for:

* Grok Build
* Antigravity CLI

The architecture should allow additional agents without modifying the core shell.

### Phase 5 — Session Manager

Implement:

```bash
nsh agent list
nsh agent attach
nsh agent stop
nsh agent restart
```

Add persistent session metadata where practical.

### Phase 6 — Harness Proxy

Introduce:

```bash
nsh proxy start
```

with a local provider-routing layer.

This should be developed independently from the PTY agent runner so users can use either feature without requiring the other.

## Design Principles

1. **Do not reimplement coding agents.** `nsh` should orchestrate existing agents.

2. **Do not treat agents as LLM providers.** Providers and harnesses are separate abstractions.

3. **Preserve native agent behavior.** Claude Code should still behave like Claude Code; Codex should still behave like Codex.

4. **PTY first.** Interactive CLI compatibility is more important than forcing every agent into a normalized protocol.

5. **Configuration over hardcoding.** New agents should be addable through the registry/configuration layer.

6. **Provider proxying should be optional.** Users should be able to use their existing subscriptions and authentication without routing through `nsh`.

7. **Security first.** Credentials should never be logged, persisted unnecessarily, or passed through the proxy without explicit configuration.

8. **Build incrementally.** Agent launching should ship before attempting full protocol normalization.

## Expected Result

After implementation, `nsh` becomes more than an AI-enhanced shell.

It becomes a Linux-native control layer for the rapidly growing ecosystem of AI coding agents:

```text
                    ┌───────────────┐
                    │      nsh      │
                    │ Agentic Shell │
                    └───────┬───────┘
                            │
             ┌──────────────┼──────────────┐
             │              │              │
          Shell           Agents         Proxy
             │              │              │
             │       ┌──────┼──────┐       │
             │       │      │      │       │
             │    Claude  Codex  OpenCode  │
             │                    │         │
             │                Grok/AGY      │
             │                              │
             └──────────────────────────────┘
```

This architecture gives `nsh` a clear path from an AI-powered shell to a universal terminal-native agent harness without coupling the project to any single model provider or coding-agent implementation.

My recommendation is to **start with Phase 1 + PTY**, not the proxy. Once `nsh agent claude/codex/opencode` works reliably, the proxy becomes a separate layer rather than a giant architectural bet.

One particularly good long-term direction is **ACP compatibility**. Zed is already using ACP as an external-agent integration mechanism, and Grok Build and other newer agents are increasingly exposing agent protocols. ([Zed][2]) If `nsh` eventually speaks ACP, you could support an expanding ecosystem without writing a bespoke adapter for every agent.

[1]: https://github.com/basharkhan7776/nsh "GitHub - Basharkhan7776/nsh: nsh is an intelligent shell that combines the power of a traditional Unix shell with AI capabilities. Like zsh, it provides a rich interactive command-line experience, but with built-in AI commands to ask questions, perform tasks, plan projects, and write code. · GitHub"
[2]: https://zed.dev/docs/ai/external-agents?utm_source=chatgpt.com "External Agents | External Agents - Zed"
[3]: https://oh.mifune.dev/docs/harnesses/t3code?utm_source=chatgpt.com "T3 Code | Open Harness"
[4]: https://github.com/anthropics/claude-code/issues/14165?utm_source=chatgpt.com "[BUG] Native binary installation ignores HTTP_PROXY for API streaming requests (Bun fetch issue ?) · Issue #14165 · anthropics/claude-code · GitHub"
[5]: https://github.com/twaldin/harness?utm_source=chatgpt.com "GitHub - twaldin/harness: Unified Python interface for invoking AI coding-agent CLIs (claude-code, opencode, codex, gemini, aider, swe-agent) as subprocesses. · GitHub"
