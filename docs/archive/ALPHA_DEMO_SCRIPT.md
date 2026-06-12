# CortexDB Alpha Demo Script

This document guides you through running the official **v0.1.0-core-alpha** demonstration workflow.

---

## 1. Quick Start (1-Minute Demo)

You can run the entire lifecycle demonstration with a single command from the repository root:

```bash
make demo
```

This runs the automated script `examples/demo/investment_projects/run.sh`, which performs:
1. Creating a temporary database.
2. Ingesting fact cells with structured metadata (project budget, currencies, legal scopes).
3. Querying individual cells.
4. Performing a cold database restart.
5. Search query matching.
6. Compiling a full **Context Pack** with anomaly reports.
7. Running a deterministic **Fact Verification** query to detect numeric conflicts (hallucination defense).

---

## 2. Interactive Tour via CLI

If you prefer to run commands manually:

### Step A: Put fact cells
```bash
cargo run -p cortex-cli -- put ./db_temp 1 "scope=project:investments\nstatus=ready\nsource=report_q1.pdf#page=3\nproject=Solar Plant\nmetric=budget\nvalue=1.2\ncurrency=KZT\n\nSolar Plant report highlights. The budget for Solar Plant in Q1 is 1.2B KZT."
```

### Step B: Run Search
```bash
cargo run -p cortex-cli -- search ./db_temp project:investments "Solar"
```

### Step C: Retrieve Context Pack (JSON Format)
```bash
cargo run -p cortex-cli -- context ./db_temp project:investments "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;" --json
```

### Step D: Verify Fact with Numeric Guards (JSON Format)
```bash
cargo run -p cortex-cli -- verify ./db_temp project:investments "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;" --json
```
