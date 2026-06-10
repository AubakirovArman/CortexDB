# LLM Inference Design

Status: future phase 2 local contract started. A deterministic test-double
endpoint exists for contract and SDK proof, but there is still no production
model runtime, GPU scheduler, remote provider integration, or built-in LLM
readiness claim.

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

Core Alpha exposes retrieval, ContextPack, verification, search, ingestion,
storage administration APIs, and an opt-in `/v1/inference` deterministic
test-double endpoint. The endpoint is disabled by default and only accepts
explicit ContextPack input. It does not call a model provider and does not
retrieve context internally.

`/v1/llm` and `/v1/chat` remain intentionally unimplemented. Any production LLM
runtime must pass the future contract, safety, smoke, secrets, resource, and
operations gates before public readiness claims.

## Resource Limits

The runtime must support request size limits, token limits, concurrency limits,
timeouts, cancellation, and queue backpressure. GPU scheduling is out of scope
until a concrete runtime is chosen.

## Runtime Safety Config

The local `LlmRuntimeSafetyConfig` contract validates the future runtime shape
before any real model provider is selected. It requires bounded prompt bytes,
bounded ContextPack cell counts, bounded output tokens, request timeout,
queue capacity, and max concurrent requests.

The contract also rejects request-body API keys and default prompt-body logging.
This keeps provider secrets in runtime environment only and preserves the
current audit boundary where full prompt bodies are not logged by default.

## Safety And Audit

Model calls must be auditable without logging secrets or full sensitive prompt
bodies by default. Safety policy must define prompt visibility, response
storage, redaction, and failure behavior.

The current `/v1/inference` test-double emits local
`llm_inference_decision` audit records when route-level audit logging is
enabled. These records contain outcome, rejection reason, tenant, principal
metadata, status, provider/model ids, ContextPack cell count, citation count,
and whether a request-body API key was present. They intentionally do not store
the prompt body, ContextPack text, citation strings, or secret values.

The response also includes an `grounding` report produced from the supplied
ContextPack. It splits the generated answer into spans and reports whether each
span is supported by packed context terms and citations. This guard is
deterministic and intentionally flags unsupported system-added text as
unsupported instead of trusting the model output.

## Prompt Visibility

The runtime must not log full prompt bodies by default. Audit records may include
route, tenant, principal, model id, token counts, latency, status, and redacted
ContextPack metadata. Full prompt capture needs an explicit debug mode and must
be unavailable for normal production operation.

Provider keys must come from runtime environment only. They must not be stored
inside ContextPack payloads, request bodies, fixtures, GitHub Actions logs, or
repository files.

## Deterministic Test Double

The first implementation includes a deterministic test-double provider for
contract and SDK proof. Enable it explicitly with:

```bash
CORTEXDB_LLM_TEST_DOUBLE=true
```

The provider accepts `provider=test_double` and `model=deterministic-echo-v1`.
It returns deterministic responses from explicit ContextPack input and never
calls an external model or requires provider credentials.

## Current Evidence Boundary

The current gates prove local prerequisites only:

| Gate | Evidence |
| --- | --- |
| `make llm-inference-contract-check` | OpenAPI and server routes expose only the disabled-by-default `/v1/inference` test-double contract; `/v1/llm` and `/v1/chat` remain absent. |
| `make llm-inference-safety-check` | The design and local runtime safety fixture contain ContextPack, AgentView, prompt-visibility, resource-limit, timeout, queue-backpressure, no-request-key, no-prompt-logging, and decision-audit rules. |
| `make llm-inference-smoke-check` | Deterministic request/response fixtures and server tests prove the test-double path without real provider calls. |
| `make secrets-check` | Tracked repository files are scanned for provider-secret-like literals. |

Reports are written under `target/llm-inference/` and keep
`built_in_llm_ready=false`. They do not claim a model runtime, scheduling layer,
remote provider integration, or production inference readiness.

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
