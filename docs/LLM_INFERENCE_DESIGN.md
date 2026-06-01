# LLM Inference Design

Status: future design gate, not implemented.

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

## Resource Limits

The runtime must support request size limits, token limits, concurrency limits,
timeouts, cancellation, and queue backpressure. GPU scheduling is out of scope
until a concrete runtime is chosen.

## Safety And Audit

Model calls must be auditable without logging secrets or full sensitive prompt
bodies by default. Safety policy must define prompt visibility, response
storage, redaction, and failure behavior.

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
