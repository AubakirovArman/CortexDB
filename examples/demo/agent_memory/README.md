# Agent Memory Demo

This demo shows the local Agent Memory v2 path:

```text
REMEMBER
-> durable memory cell with TTL
-> ContextPack retrieve
-> VERIFY FACT over memory
-> engine TTL/decay/feedback tests
```

Run:

```bash
make agent-memory-demo-check
```

or:

```bash
examples/demo/agent_memory/run.sh
```

The demo writes to:

```text
target/agent-memory-demo/db
```

It proves a local developer flow only. It does not claim production memory
ranking, natural-language contradiction extraction, or hosted agent runtime
behavior.
