# EXOCORTEX Architecture

## Design thesis

EXOCORTEX should not be a pile of agents talking to each other. The first architectural unit is a **cognitive kernel** that owns state, provenance, permissions, and the lifecycle of cognition.

Model providers are replaceable intelligence adapters. They may propose reasoning, plans, critiques, and simulations, but they do not own canonical memory or permission to perform consequential actions.

## Planes

```text
HUMAN PLANE
  intent | values | approval | correction
                 |
                 v
TRUSTED PLANE (Rust)
  event log
  canonical memory
  provenance
  epistemic labels
  policy / approval
  orchestration
  audit
                 |
                 v
INTELLIGENCE PLANE
  LLMs
  retrieval
  critics
  planners
  simulation
                 |
                 v
ACTION PLANE
  files | email | browser | code | devices
```

## V0 invariants

1. **Inference is not fact.** Every persisted memory carries an epistemic type.
2. **Memory has provenance.** A memory can be traced to the event that created it.
3. **Plan is not permission.** External writes and high-impact actions require explicit human approval.
4. **Models do not own state.** The kernel owns canonical state; models return proposals.
5. **Frameworks are adapters.** The system must not depend on a single agent framework or model vendor.

## Current modules

### Cognitive events

An appendable stream of observations, user statements, action proposals, outcomes, and reflections.

### Memory

V0 supports four categories:

- working
- episodic
- semantic
- prospective

Every record also carries an epistemic label:

- observation
- user-confirmed fact
- inference
- hypothesis
- prediction

### Permission boundary

Actions are classified into:

- internal
- read-only
- reversible
- external write
- high impact

External writes and high-impact actions enter a pending state until the human approves or rejects them.

## Next milestones

### V0.2 — persistence

Move the in-memory stores behind traits and add SQLite/PostgreSQL persistence without changing the public kernel semantics.

### V0.3 — model adapter

Add a provider-neutral reasoning interface. Model output must be structured and converted into typed proposals before it can touch state.

### V0.4 — retrieval

Add hybrid retrieval: structured filters + full-text + embeddings. Vector similarity must not be treated as truth.

### V0.5 — cognitive loop

Implement a bounded loop:

```text
observe -> retrieve -> reason -> critique -> propose -> approve -> act -> verify -> reflect
```

Each transition emits an event so the entire cognitive trace is inspectable.

### V1 — research harness

Create controlled experiments comparing:

1. no AI
2. conventional chatbot
3. memory-enabled assistant
4. EXOCORTEX cognitive kernel

Primary outcome metrics should focus on human performance and agency, not token generation.
