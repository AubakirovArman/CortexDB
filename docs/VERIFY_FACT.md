# VERIFY FACT Design

CortexDB provides deterministic fact verification through the `VERIFY FACT` AQL statement and the `/v1/verify` HTTP endpoint.

## Purpose

Unlike LLM-based verification (probabilistic), CortexDB verification is **deterministic**:

1. Parses numeric claims from both the fact and stored cell payloads.
2. Detects contradictions using integer-only arithmetic.
3. Returns structured evidence with citations.

## NumericValue Model

The core of verification is the `NumericValue` struct:

```rust
pub struct NumericValue {
    pub raw: String,           // Original text
    pub scaled_value: u64,     // Normalized integer
    pub currency: Option<String>, // KZT, USD, EUR, etc.
    pub unit: Option<String>,  // m, kg, %, etc.
    pub magnitude: Option<Magnitude>, // Billion, Million, Thousand, Percent
}
```

### Magnitude Suffixes

| Suffix | Scaled Value |
|--------|-------------|
| `B`, `billion`, `млрд` | × 1,000,000,000 |
| `M`, `million`, `млн` | × 1,000,000 |
| `K`, `thousand`, `тыс` | × 1,000 |
| `%`, `percent` | × 1 |

### Example Parsing

| Input | scaled_value | currency | magnitude |
|-------|-------------|----------|-----------|
| `1.2B KZT` | 1,200,000,000 | KZT | Billion |
| `1.5M USD` | 1,500,000 | USD | Million |
| `15K m` | 15,000 | — | Thousand |
| `12.5%` | 12 | — | Percent |

## Conflict Detection

Two numeric values **conflict** when:

1. They have the same currency or unit context.
2. Their `scaled_value` differs.

## Verification Verdicts

| Status | Meaning |
|--------|---------|
| `supported` | Evidence found, no contradictions |
| `contradicted` | No evidence, but contradictions found |
| `mixed_evidence` | Both supporting and contradicting evidence |
| `insufficient` | No relevant evidence found |

## Guards

Verification may emit guards (warnings):

| Code | Meaning |
|------|---------|
| `missing_citation` | Evidence lacks source reference |
| `numeric_mismatch` | Payload number contradicts fact number |

## Usage

```bash
cargo run -p cortex-cli -- verify ./data project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;'
```

Or via HTTP:

```bash
curl -X POST 'http://127.0.0.1:8090/v1/verify?scope=project:investments' \
  -d 'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;'
```
