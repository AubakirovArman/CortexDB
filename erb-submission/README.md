# CortexDB EnterpriseRAG-Bench Package

Status: single official interim result for the current package. This is not a
leaderboard claim because the official `gpt-5.4` judge has not been run yet.

Full 500 questions, no-oracle inference, Gemma answerer, Gemini judge:

| Metric | Value |
| --- | ---: |
| Overall combined correctness/completeness | **47.74** |
| Correctness | 50.0% |
| Completeness | 53.7% |
| Document recall | 55.71% |
| Invalid extra docs | 9.23 |

System: CortexDB `engine-aql` retrieval with `weighted` rerank, answerer
`google/gemma-4-31B-it`, prompt `official-clean-v1`, context mode
`question-window-digest-ranked`, active document budget up to 8 docs and 8000
chars per document. No oracle metadata is available to retrieval or answer
generation.

Honesty note: `gemini-3.5-flash` is the single current judge for this package.
The final leaderboard-comparable number must be produced by re-judging these
same `answers.jsonl` rows with the official `gpt-5.4` evaluator when access is
available. Older mixed-judge numbers are intentionally excluded from this
package headline.

## Files

| File | What |
| --- | --- |
| `answers.jsonl` | 500 Gemma answers (`question_id`, `answer`, `document_ids`). |
| `official_results.json` | Current single official interim result, Gemini judge, Overall 47.74. |
| `oracle_usage_audit.json` | Audit of clean questions, retrieval, answers, and scripts. |
| `official_clean_gate_report.json` | Clean input/retrieval gate report. |
| `config_answer_report.json` | Gemma answer-generation provenance. |
| `REPRODUCE.md` | Reproduction and re-judge guide. |

Repo: https://github.com/AubakirovArman/CortexDB
Contact for submission: joachim@onyx.app
