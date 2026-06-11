# CortexDB — EnterpriseRAG-Bench submission package

**Headline (leaderboard metric = Overall = combined correctness×completeness),
full 500 questions, no-oracle, scored with the benchmark's own
`metrics_based_eval.py`:**

| judge | Overall | Correctness | Completeness | Doc recall |
| --- | ---: | ---: | ---: | ---: |
| **gpt-5.2** (leaderboard-comparable; proxy for official gpt-5.4) | **43.27** | 49.2 | 54.2 | 85.8 |
| gemini-3.5-flash (lenient self-judge — upper bound) | 48.75 | 53.6 | 59.2 | 85.7 |

System: CortexDB (from-scratch Rust agent-native context DB), `engine-aql`
retrieval + `weighted` rerank fused with bge-m3 dense (doc recall 85.8%);
answerer `gemini-3.5-flash`. No oracle metadata at inference.

**Judge note:** the official evaluator defaults to `gpt-5.4`; our key has only
`gpt-5.2`, so the comparable number is the gpt-5.2 column (43.27). Re-run §6 of
`REPRODUCE.md` with `gpt-5.4` for the fully-official score.

## Files

| File | What |
| --- | --- |
| `answers.jsonl` | 500 submission answers (`question_id`, `answer`, `document_ids`). |
| `official_results_gpt5.2_judge.json` | Official evaluator output, gpt-5.2 judge (Overall 43.27). |
| `official_results_gemini35_judge.json` | Official evaluator output, gemini-3.5 judge (Overall 48.75). |
| `questions_updated_gpt5.2.jsonl` | Evaluator's consensus document-correction output (3 corrected). |
| `oracle_usage_audit.json` | Audit: 0 oracle/gold fields in clean inference artifacts. |
| `official_clean_gate_report.json` | No-oracle clean-gate report. |
| `config_answer_report.json` | Answer-generation config provenance. |
| `REPRODUCE.md` | Full from-scratch reproduction guide. |

Repo: https://github.com/AubakirovArman/CortexDB
Contact for submission: joachim@onyx.app
