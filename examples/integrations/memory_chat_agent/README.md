# Memory Chat Agent

This example turns the existing Agent Memory demo into a first-class user flow:

1. Load a small CortexDB fixture.
2. Store an agent preference with `REMEMBER ... TTL 3600 SECONDS`.
3. Retrieve context for the next turn.
4. Run `VERIFY FACT` before the mock chat agent answers.

Run:

```bash
python3 examples/integrations/memory_chat_agent/demo.py --self-test
```

The example uses a mock chat model. The database operations are real local
CortexDB CLI calls.
