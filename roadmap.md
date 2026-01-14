# Agentic AI Linux (Dextro) - Technical Roadmap

## 1. Executive Summary

**Project Dextro** aims to replace the traditional deterministic command-line interface (CLI) with an **Intent-Driven Execution Environment (IDEE)**. The core component, **Neuro-Shell (nsh)**, utilizes a Large Action Model (LAM) to interpret natural language, resolve dependencies, and ensure safety through probabilistic risk assessment.

## 2. Technical Architecture

### 2.1 Core Components

- **Neuro-Shell Daemon (`nshd`)**: Background service maintaining a "Memory Vector" of the last 10,000 interactions.
- **Intent Resolution Layer (IRL)**: Middleware between TTY and Kernel for "Pre-Cognitive" input analysis.
- **Heuristic Piping (`|>`)**: Replaces byte streams with structured Semantic Data Flows.
- **Smart-Sudo Protocol (SSP)**: Dynamic risk assessment engine replacing binary privilege escalation.

### 2.2 Intent Resolution Layer (IRL)

The IRL operates on a "Confidence-to-Execution" curve to decouple typing latency from execution.

- **Layer 0 (Semantic Parser)**: Tokenizes natural language mixed with bash syntax.
- **Layer 1 (Context Vectorizer)**: Weighs commands against the `nshd` Memory Vector.
- **Layer 2 (Action Synthesizer)**: Constructs the JSON payload for the Kernel Action Dispatcher.

**Performance Target**: Mean Time to Intent (MTTI) of **14.2ms**.

### 2.3 Heuristic Piping Protocol (`|>`)

Standard pipes (`|`) transfer unstructured text. Heuristic pipes (`|>`) initiate **Dynamic Schema Negotiation (DSN)**.

- **Mechanism**:
  1.  **Source Introspection**: Identify output type (JSON, CSV, Binary).
  2.  **Destination Requirement**: Analyze input expectation of the receiving command.
  3.  **Auto-Transpilation**: Inject logic to convert formats (e.g., JSON -> Filter Query).
- **Semantic Types**: `T_GEO` (Coordinates), `T_NET` (IP/CIDR), `T_ERR` (Stack Traces), `T_IMG` (Raster).

### 2.4 Smart-Sudo Protocol (SSP)

Replaces password prompts with a **Dynamic Risk Score (DRS)**.

**Risk Formula**:
$$ R*{cmd} = \frac{(Impact*{fs} \times Sensitivity*{data})}{Trust*{user}} $$

**Authorization Tiers**:

- **Low ($< 0.3$)**: Silent Auto-approval.
- **Medium ($0.3 - 0.7$)**: User Confirmation Required.
- **High ($> 0.7$)**: Mandatory Sandbox Dry-Run + Biometric/Key Auth.

---

## 3. Development Roadmap

### Phase 1: Core Shell Foundation (Rust)

- **Objective**: Build a high-performance, async REPL capable of sub-millisecond distinct rendering.
- **Tech Stack**: Rust, `tokio` (Async Runtime), `reedline` (Line Editor), `serde`.
- **Deliverables**:
  - [ ] Basic Shell Loop (Input/Parsing/Execution).
  - [ ] Custom Tokenizer (handling mixed NLP/Bash syntax).
  - [ ] `nshd` Daemon stub for session history.

### Phase 2: The Agentic Layer (IRL Integration)

- **Objective**: Implement "Pre-Cognitive" Input Buffering.
- **Deliverables**:
  - [ ] **Shadow Buffer**: A background thread analyzing keypresses in real-time.
  - [ ] **Intent Engine**: Integration with a local quantized LLM (e.g., Llama-3-8B via `candle` or `ollama-rs`).
  - [ ] **Ghost Text**: UI for displaying low-latency speculative execution results.

### Phase 3: Semantic Transport System

- **Objective**: Implement the `|>` operator and `StreamWeaver` logic.
- **Deliverables**:
  - [ ] **Type Inference Engine**: Logic to detect output schemas of common tools (`ls`, `curl`, `git`).
  - [ ] **Transpilers**: Built-in JSON-to-Text and Text-to-JSON adaptors.
  - [ ] **Pipe Logic**: Rust channel-based object passing between process structs.

### Phase 4: Security & Hallucination Dampeners

- **Objective**: Implement Smart-Sudo and safety guardrails.
- **Deliverables**:
  - [ ] **Risk Analyzer**: Regex and Heuristic evaluation of command strings.
  - [ ] **Dry-Run Container**: Integration with namespaces (`unshare`) or Docker for ephemeral command testing.
  - [ ] **Hallucination Dampener**: Cross-reference generated flags with local `man` pages database.

### Phase 5: Hardware Acceleration (The "T3Y" Bridge)

- **Objective**: Offload inference and rendering to GPU (Simulated/Prototype).
- **Deliverables**:
  - [ ] GPU-accelerated terminal rendering pipeline (via `wgpu`).
  - [ ] Dedicated Inference thread pinned to NPU/GPU if available.

---

## 4. Operational Metrics Targets

| Metric                | Legacy Shell (Bash/Zsh) | Dextro (nsh) Target       |
| :-------------------- | :---------------------- | :------------------------ |
| **Parsing Overhead**  | 22% CPU                 | **4% CPU**                |
| **Typing Latency**    | 12ms                    | **1.5ms**                 |
| **Context Retention** | Session Only            | **Last 10k Interactions** |
| **Apps Per Second**   | N/A                     | **450**                   |
