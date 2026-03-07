# nsh - AI-Powered Shell

nsh is an intelligent shell that combines the power of a traditional Unix shell with AI capabilities. Like zsh, it provides a rich interactive command-line experience, but with built-in AI commands to ask questions, perform tasks, plan projects, and write code.

## Features

- **AI-Powered Commands**: Built-in commands (`ask`, `do`, `plan`, `build`) that leverage AI to assist with various tasks
- **Syntax Highlighting**: Color-coded commands for better readability
- **Intelligent Completion**: File and command completion with history hints
- **Traditional Shell Functions**: All the familiar shell commands you expect

## AI Commands

| Command | Description |
|---------|-------------|
| `ask <question>` | Ask any question and get AI-generated responses |
| `do <task>` | Have AI execute a task on your system |
| `plan <goal>` | Plan and outline steps to achieve a goal |
| `build <project>` | Generate code and build projects |

## Installation

```bash
cargo build --release
```

## Usage

Start the shell:
```bash
./target/release/nsh
```

### Basic Commands

```bash
# Ask a question
nsh: ~ ask what is rust

# Execute a task
nsh: ~ do create a new directory called test

# Plan a project
nsh: ~ plan build a web application

# Generate code
nsh: ~ build a simple rest api in python

# Standard shell commands
nsh: ~ ls -la
nsh: ~ cd /path/to/dir
nsh: ~ exit
```

## Requirements

- Rust (latest stable)
- Cargo

## License

MIT
