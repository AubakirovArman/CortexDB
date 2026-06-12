enterprise-rag-bench-official-repo:
	@if [ ! -d "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)/.git" ]; then \
	  git clone --depth 1 https://github.com/onyx-dot-app/EnterpriseRAG-Bench "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)"; \
	else \
	  git -C "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" pull --ff-only; \
	fi

enterprise-rag-bench-official-env: enterprise-rag-bench-official-repo $(ENTERPRISE_RAG_BENCH_VENV)/.requirements.stamp

$(ENTERPRISE_RAG_BENCH_VENV)/.requirements.stamp: $(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)/requirements.txt
	python3 -m venv "$(ENTERPRISE_RAG_BENCH_VENV)"
	"$(ENTERPRISE_RAG_BENCH_PYTHON)" -m pip install --upgrade pip
	"$(ENTERPRISE_RAG_BENCH_PYTHON)" -m pip install -r "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)/requirements.txt"
	touch "$(ENTERPRISE_RAG_BENCH_VENV)/.requirements.stamp"

enterprise-rag-bench-preflight: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/preflight.py \
	  --bench-root "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --report "$(ENTERPRISE_RAG_BENCH_PREFLIGHT_REPORT)"

enterprise-rag-bench-balanced-50: enterprise-rag-bench-preflight
	python3 scripts/enterprise_rag_bench/build_balanced_subset.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --limit "$(ENTERPRISE_RAG_BENCH_SUBSET_LIMIT)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_SUBSET_ROOT)" \
	  --output-prefix "$(ENTERPRISE_RAG_BENCH_SUBSET_PREFIX)"

enterprise-rag-bench-balanced-100: enterprise-rag-bench-preflight
	python3 scripts/enterprise_rag_bench/build_balanced_subset.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --limit "100" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_SUBSET_ROOT)" \
	  --output-prefix "$(ENTERPRISE_RAG_BENCH_SUBSET_100_PREFIX)"

