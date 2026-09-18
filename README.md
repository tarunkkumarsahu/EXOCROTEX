# EXOCORTEX

EXOCORTEX is an experimental human cognitive-extension system. Its goal is to increase useful human cognitive bandwidth—memory, reasoning, planning, attention, and decision support—while keeping the human in control of consequential actions.

This repository is being built as a **cognitive systems platform**, not as a chatbot wrapper.

## Core principle

> Autonomous cognition, permissioned action.

The system may remember, retrieve, reason, critique, simulate, and prepare actions. External or high-impact actions must cross an explicit human-approval boundary.

## V0

The first milestone is a small, typed Rust cognitive kernel with:

- cognitive events and provenance
- multiple memory types
- epistemic labels (fact vs inference vs hypothesis vs prediction)
- action-risk classification
- human-approval gating
- an interactive CLI for exercising the kernel
- tests and CI

LLM providers, vector retrieval, PostgreSQL, Python experiments, and a desktop UI come later. The kernel should stay useful even if any model provider or agent framework changes.

## Architecture

```text
Human
  |
  v
Interface / CLI
  |
  v
+-----------------------------+
| Rust Cognitive Kernel       |
| events | memory | policy    |
| state  | approval | audit   |
+-------------+---------------+
              |
      intelligence adapters
              |
      LLMs / Python / tools
```

Rust owns trusted state, permissions, event semantics, and orchestration. Python will own fast AI/ML research and evaluation. TypeScript/Tauri can own the human interface. PostgreSQL can become the canonical persistent store.

Read the deeper architecture notes in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Run V0

Install stable Rust, then:

```bash
cargo check --workspace
cargo test --workspace
cargo run -p exo-cli
```

Inside the CLI:

```text
:remember Our first experiment compares chatbot vs cognitive kernel.
:recall experiment
:propose Send the experiment decision to professor
:pending
:approve <proposal-uuid>
:status
```

The external-write proposal will remain blocked until explicit approval.

## Repository layout

```text
apps/
  exo-cli/             interactive kernel shell

crates/
  cognitive-core/      events, memory, policy, kernel

docs/
  ARCHITECTURE.md      design invariants and roadmap

.github/workflows/
  ci.yml               fmt, check, clippy, tests
```

## Status

**V0.1 foundation implemented on the cognitive-kernel feature branch.**

Current scope is deliberately small: prove the kernel semantics first, then add persistence, model adapters, retrieval, and the full cognitive loop.
