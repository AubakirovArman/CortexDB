# CortexDB root Makefile.
#
# Keep this file as a small index. Add variables and targets to mk/*.mk files
# by domain, and keep behavior-preserving moves separate from logic changes.

include mk/phony.mk
include mk/vars-core.mk
include mk/vars-external-benchmarks.mk
include mk/vars-enterprise-rag.mk
include mk/vars-ops-release.mk
include mk/core.mk
include mk/longmemeval.mk
include mk/enterprise-rag-core.mk
include mk/enterprise-rag-quality.mk
include mk/enterprise-rag-runs.mk
include mk/external-benchmarks.mk
include mk/performance-dashboard.mk
include mk/ann.mk
include mk/storage-ops.mk
include mk/release.mk
