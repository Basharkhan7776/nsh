# Running Agentic AI Linux (nsh) Locally

This guide explains how to set up and run the **Neuro-Shell (nsh)** in a virtualized development environment using Docker. This ensures safe testing of "Smart-Sudo" and agentic capabilities without risking your host system.

## Prerequisites

- **Docker** installed on your machine.
- **Gemini API Key** (Required for the Large Action Model).
- **Google Custom Search API Key & CSE ID** (Optional, for web search).

## 1. Environment Setup

### 1.1 Configure API Keys
Copy the example environment file and add your keys:

```bash
cp .env.example .env
# Open .env and paste your GEMINI_API_KEY, GOOGLE_API_KEY, and GOOGLE_CSE_ID
```

### 1.2 Build the Virtual Environment
We use a Docker container to simulate the Linux terminal environment.

```bash
make dev-env
```
*This command builds the `nsh-env` image and drops you into a bash shell inside the container.*

## 2. Running nsh

Once inside the container (or if you have Rust installed locally), compile and run `nsh`.

### 2.1 Initialization
First, ingest the project documentation (or any text files) into the local vector database.

```bash
cargo run -- init
```
*This processes `document.txt` and `roadmap.md`, generating embeddings for the RAG system.*

### 2.2 Interactive REPL (Zsh-like Mode)
To enter the interactive shell mode:

```bash
cargo run
```

You will see the `nsh ➜` prompt. 

**Supported Modes:**
- **Standard Command**: Just type `ls`, `grep`, etc.
- **Ask (`?`)**: Query the documentation/web.
  - `? How does Smart-Sudo work?`
- **Do (`!`)**: Execute natural language intent.
  - `! Find all large files in /tmp and delete them`
- **Remember (`+`)**: Save a fact to memory.
  - `+ I prefer using vim for editing.`

### 2.3 Single Command Mode
You can also run specific agentic tasks directly from the standard terminal:

**Ask a Question:**
```bash
cargo run -- ask "What is the Heuristic Pipe operator?"
```

**Execute Intent (Agentic):**
```bash
cargo run -- do "List all files in src and show their size"
```
*Note: High-risk commands (e.g., involving `rm` or `sudo`) will trigger the Smart-Sudo sandbox simulation.*

**Remember Context:**
```bash
cargo run -- remember "Project codename is Cortex."
```

## 3. Features Tested

- **RAG (Retrieval Augmented Generation)**: Queries local docs + Google Search.
- **Smart-Sudo Sandbox**: Try running `! delete everything in /` to see the risk assessment intercept the command and simulate a sandbox dry-run.
- **Agentic Execution**: Natural language to Bash translation.
