# LLM Inference Design

Status: future phase 1 local evidence gates started, no model runtime or
inference endpoint implemented.

## Goal

Decide whether CortexDB should host model inference directly and define the
minimum safe architecture if it does.

## Build-vs-integrate Decision

The default product boundary remains ContextPack generation for external agent
and model runtimes. Built-in inference should be added only if it improves a
specific user workflow enough to justify runtime, security, and cost ownership.

## Provider Interface

The interface must support deterministic test doubles, local runtimes, and
OpenAI-compatible remote providers. Real provider keys must come from runtime
environment only and must never be required for CI.

## ContextPack Boundary

Inference must consume explicit ContextPack input. It cannot bypass AgentView,
tenant routing, AQL policy, or source citation requirements.

## API Contract Boundary

Core Alpha exposes retrieval, ContextPack, verification, search, ingestion, and
storage administration APIs. It does not expose `/v1/inference`, `/v1/llm`, or
`/v1/chat`.

Any future inference endpoint must be opt-in, disabled by default, documented in
the OpenAPI contract before implementation, and backed by typed request and
response structs. Until then, ContextPack remains the product boundary for LLM
consumers.

## Resource Limits

The runtime must support request size limits, token limits, concurrency limits,
timeouts, cancellation, and queue backpressure. GPU scheduling is out of scope
until a concrete runtime is chosen.

## Safety And Audit

Model calls must be auditable without logging secrets or full sensitive prompt
bodies by default. Safety policy must define prompt visibility, response
storage, redaction, and failure behavior.

## Prompt Visibility

The runtime must not log full prompt bodies by default. Audit records may include
route, tenant, principal, model id, token counts, latency, status, and redacted
ContextPack metadata. Full prompt capture needs an explicit debug mode and must
be unavailable for normal production operation.

Provider keys must come from runtime environment only. They must not be stored
inside ContextPack payloads, request bodies, fixtures, GitHub Actions logs, or
repository files.

## Deterministic Test Double

The first implementation must include a deterministic test-double provider for
CI. This provider returns fixture-backed responses from explicit ContextPack
input and never calls an external model or requires provider credentials.

## Current Evidence Boundary

The current gates prove local prerequisites only:

| Gate | Evidence |
| --- | --- |
| `make llm-inference-contract-check` | OpenAPI and server routes do not expose future inference endpoints, and API docs keep the no-endpoint boundary explicit. |
| `make llm-inference-safety-check` | The design contains ContextPack, AgentView, prompt-visibility, resource-limit, timeout, and queue-backpressure rules. |
| `make llm-inference-smoke-check` | Deterministic request/response fixtures prove the shape of a future test-double smoke path without real provider calls. |
| `make secrets-check` | Tracked repository files are scanned for provider-secret-like literals. |

Reports are written under `target/llm-inference/` and keep
`built_in_llm_ready=false`. They do not claim a model runtime, scheduling layer,
or inference endpoint exists.

## Required Gates

1. `make llm-inference-design-check`
2. `make llm-inference-contract-check`
3. `make llm-inference-safety-check`
4. `make llm-inference-smoke-check`
5. `make secrets-check`

## Acceptance

1. Inference is explicitly enabled or disabled.
2. ContextPack remains the source of retrieved context.
3. CI passes without real provider keys.
4. Audit and quota behavior is documented.

## Non-goals

1. Training or fine-tuning models.
2. Committing provider keys.
3. Hidden retrieval expansion outside AgentView.
4. Production GPU scheduling before runtime ownership is defined.
