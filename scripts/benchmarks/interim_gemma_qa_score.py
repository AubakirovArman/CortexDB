#!/usr/bin/env python3
"""Interim QA scorer: judge generated hypotheses via an OpenAI-compatible endpoint
(Gemma on the LiteLLM proxy) using the OFFICIAL LongMemEval anscheck rubric.

The judge model is an INTERIM stand-in for the leaderboard-official judge
(GPT-4o for LongMemEval, gpt-5.4 for ERB) — every result is stamped
`leaderboard_comparable: false`. The anscheck prompt templates below are copied
VERBATIM from target/external-benchmarks/longmemeval/src/evaluation/evaluate_qa.py
(that module's own import chain needs backoff/openai/numpy, absent here); the
prompt text is byte-identical, so the rubric is faithful.

  - FAST (`--self-test`): validate the rubric selects a distinct, non-empty prompt
    per question type + the accuracy aggregation, with a stub judge (no network).
  - REAL: `--hyp <hypotheses.jsonl> --model <id> --base-url <proxy>/v1
    --key-file <key>` (+ optional `--ref` for LongMemEval, `--type-field`,
    `--gold-field`, `--rubric longmemeval|generic`).
"""
from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys
import time
import urllib.error
import urllib.request


# Copied VERBATIM from evaluate_qa.py::get_anscheck_prompt (byte-identical text).
def get_anscheck_prompt(task, question, answer, response, abstention=False):
    if not abstention:
        if task in ['single-session-user', 'single-session-assistant', 'multi-session']:
            template = "I will give you a question, a correct answer, and a response from a model. Please answer yes if the response contains the correct answer. Otherwise, answer no. If the response is equivalent to the correct answer or contains all the intermediate steps to get the correct answer, you should also answer yes. If the response only contains a subset of the information required by the answer, answer no. \n\nQuestion: {}\n\nCorrect Answer: {}\n\nModel Response: {}\n\nIs the model response correct? Answer yes or no only."
            prompt = template.format(question, answer, response)
        elif task == 'temporal-reasoning':
            template = "I will give you a question, a correct answer, and a response from a model. Please answer yes if the response contains the correct answer. Otherwise, answer no. If the response is equivalent to the correct answer or contains all the intermediate steps to get the correct answer, you should also answer yes. If the response only contains a subset of the information required by the answer, answer no. In addition, do not penalize off-by-one errors for the number of days. If the question asks for the number of days/weeks/months, etc., and the model makes off-by-one errors (e.g., predicting 19 days when the answer is 18), the model's response is still correct. \n\nQuestion: {}\n\nCorrect Answer: {}\n\nModel Response: {}\n\nIs the model response correct? Answer yes or no only."
            prompt = template.format(question, answer, response)
        elif task == 'knowledge-update':
            template = "I will give you a question, a correct answer, and a response from a model. Please answer yes if the response contains the correct answer. Otherwise, answer no. If the response contains some previous information along with an updated answer, the response should be considered as correct as long as the updated answer is the required answer.\n\nQuestion: {}\n\nCorrect Answer: {}\n\nModel Response: {}\n\nIs the model response correct? Answer yes or no only."
            prompt = template.format(question, answer, response)
        elif task == 'single-session-preference':
            template = "I will give you a question, a rubric for desired personalized response, and a response from a model. Please answer yes if the response satisfies the desired response. Otherwise, answer no. The model does not need to reflect all the points in the rubric. The response is correct as long as it recalls and utilizes the user's personal information correctly.\n\nQuestion: {}\n\nRubric: {}\n\nModel Response: {}\n\nIs the model response correct? Answer yes or no only."
            prompt = template.format(question, answer, response)
        else:
            raise NotImplementedError
    else:
        template = "I will give you an unanswerable question, an explanation, and a response from a model. Please answer yes if the model correctly identifies the question as unanswerable. The model could say that the information is incomplete, or some other information is given but the asked information is not.\n\nQuestion: {}\n\nExplanation: {}\n\nModel Response: {}\n\nDoes the model correctly identify the question as unanswerable? Answer yes or no only."
        prompt = template.format(question, answer, response)
    return prompt


GENERIC_PROMPT = (
    "Question: {q}\nCorrect answer: {a}\nModel response: {r}\n\n"
    "Is the model response correct (contains the correct answer, or correctly "
    "abstains if unanswerable)? Answer yes or no only."
)

LONGMEMEVAL_TYPES = [
    "single-session-user", "single-session-assistant", "multi-session",
    "temporal-reasoning", "knowledge-update", "single-session-preference",
]


def build_prompt(rubric, qtype, qid, question, answer, hyp):
    if rubric == "longmemeval":
        return get_anscheck_prompt(qtype, question, answer, hyp, abstention="_abs" in str(qid))
    return GENERIC_PROMPT.format(q=question, a=answer, r=hyp)


def chat(url, key, model, prompt, max_tokens=10):
    body = json.dumps({"model": model, "messages": [{"role": "user", "content": prompt}],
                       "max_tokens": max_tokens, "temperature": 0}).encode()
    req = urllib.request.Request(url, data=body, headers={"Authorization": "Bearer " + key,
                                                          "Content-Type": "application/json"})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=90) as r:
                return json.loads(r.read().decode())["choices"][0]["message"]["content"].strip()
        except urllib.error.HTTPError as e:
            if e.code in {429, 500, 502, 503, 504} and attempt < 3:
                time.sleep(min(20, 2 ** attempt))
                continue
            raise
    raise RuntimeError("unreachable retry state")


def score(hyps, refs, rubric, type_field, gold_field, judge):
    """`judge(prompt) -> str`. Returns the summary dict."""
    per_type = collections.defaultdict(lambda: [0, 0])
    for h in hyps:
        qid = h["question_id"]
        ref = refs.get(qid, h)
        qtype = ref.get(type_field) or h.get(type_field) or "unknown"
        question = ref.get("question") or h.get("question", "")
        answer = ref.get("answer") or h.get(gold_field, "")
        verdict = judge(build_prompt(rubric, qtype, qid, question, answer, h.get("hypothesis", "")))
        ok = "yes" in verdict.lower()
        per_type[qtype][0] += int(ok)
        per_type[qtype][1] += 1
    total_ok = sum(v[0] for v in per_type.values())
    total = sum(v[1] for v in per_type.values())
    return {
        "leaderboard_comparable": False,
        "note": "INTERIM judge over the official rubric; not the leaderboard-official judge",
        "overall_accuracy": round(total_ok / total, 4) if total else 0.0,
        "total": total,
        "per_type": {t: {"correct": v[0], "count": v[1], "accuracy": round(v[0] / v[1], 4)}
                     for t, v in sorted(per_type.items())},
    }


def read_jsonl(path):
    return [json.loads(l) for l in pathlib.Path(path).read_text(encoding="utf-8").splitlines() if l.strip()]


def run_self_test() -> int:
    failures = []
    # (1) Every LongMemEval type + abstention yields a distinct, non-empty prompt.
    prompts = {t: build_prompt("longmemeval", t, "q1", "Q", "A", "R") for t in LONGMEMEVAL_TYPES}
    prompts["_abs"] = build_prompt("longmemeval", "single-session-user", "q_abs", "Q", "A", "R")
    for k, p in prompts.items():
        if not p or "R" not in p:
            failures.append(f"prompt for {k} is empty / missing response")
    if prompts["temporal-reasoning"] == prompts["knowledge-update"]:
        failures.append("temporal vs knowledge-update rubric not distinct")
    if "unanswerable" not in prompts["_abs"]:
        failures.append("abstention rubric not selected for _abs question_id")
    # (2) Aggregation math with a stub judge (yes iff hypothesis == gold).
    hyps = [
        {"question_id": "a", "question_type": "multi-session", "question": "q", "gold_answer": "x", "hypothesis": "x"},
        {"question_id": "b", "question_type": "multi-session", "question": "q", "gold_answer": "y", "hypothesis": "z"},
        {"question_id": "c", "question_type": "temporal-reasoning", "question": "q", "gold_answer": "p", "hypothesis": "p"},
    ]
    stub = lambda prompt: "yes" if any(f"Model Response: {g}\n" in prompt for g in ["x", "p"]) else "no"
    summary = score(hyps, {}, "longmemeval", "question_type", "gold_answer", stub)
    if summary["overall_accuracy"] != round(2 / 3, 4):
        failures.append(f"overall accuracy {summary['overall_accuracy']} != 2/3")
    if summary["per_type"]["multi-session"] != {"correct": 1, "count": 2, "accuracy": 0.5}:
        failures.append(f"multi-session agg wrong: {summary['per_type']['multi-session']}")
    if summary["leaderboard_comparable"] is not False:
        failures.append("must stamp leaderboard_comparable=false")
    if failures:
        print("interim-gemma-qa-score self-test FAILED:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print("interim-gemma-qa-score self-test passed: official rubric verbatim + distinct per type; "
          "aggregation correct; leaderboard_comparable=false.")
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--hyp")
    ap.add_argument("--ref", default="")
    ap.add_argument("--type-field", default="question_type")
    ap.add_argument("--gold-field", default="gold_answer")
    ap.add_argument("--rubric", choices=["longmemeval", "generic"], default="generic")
    ap.add_argument("--model")
    ap.add_argument("--base-url")
    ap.add_argument("--key-file")
    ap.add_argument("--output", default="")
    # Reasoning judges (e.g. gpt-oss-120B) spend tokens on chain-of-thought before
    # the yes/no verdict; 10 is enough for a plain model but starves a reasoner
    # (empty content -> parsed as "no"). Raise for reasoning judges; default keeps
    # the plain-model / self-test behaviour byte-identical.
    ap.add_argument("--max-tokens", type=int, default=10)
    args = ap.parse_args(argv)
    if args.self_test:
        return run_self_test()
    for req in ("hyp", "model", "base_url", "key_file"):
        if not getattr(args, req):
            ap.error(f"real scoring needs --{req.replace('_','-')} (or pass --self-test)")
    key = pathlib.Path(args.key_file).read_text(encoding="utf-8").strip()
    url = args.base_url.rstrip("/") + "/chat/completions"
    hyps = read_jsonl(args.hyp)
    refs = {r["question_id"]: r for r in read_jsonl(args.ref)} if args.ref else {}
    summary = score(hyps, refs, args.rubric, args.type_field, args.gold_field,
                    lambda p: chat(url, key, args.model, p, args.max_tokens))
    summary["judge_model"] = args.model
    text = json.dumps(summary, indent=2, sort_keys=True)
    print(text)
    if args.output:
        pathlib.Path(args.output).parent.mkdir(parents=True, exist_ok=True)
        pathlib.Path(args.output).write_text(text + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
