import io
import unittest
import urllib.error

from cortexdb_client import (
    AnnEvaluationResponse,
    ContextPackResponse,
    CortexDBClient,
    CortexDBError,
    VerificationReportResponse,
    SearchResponse,
)


class CortexDBClientPathTests(unittest.TestCase):
    def test_aql_builders_output_stable_statements(self) -> None:
        retrieve = CortexDBClient.build_retrieve_context_aql(
            'budget "audit"\nline',
            "investment_projects",
            mode="balanced",
            budget_tokens=2048,
            limit_candidates=10,
            where_clause='space = project:investments AND status = "ready"',
            require_citations=True,
            min_confidence="0.80",
            source_trust="0.90",
            freshness_seconds=86400,
        )
        verify = CortexDBClient.build_verify_fact_aql(
            "Solar Plant budget is 1.2B KZT",
            "investment_projects",
        )
        remember = CortexDBClient.build_remember_aql(
            "Use conservative budget assumptions",
            "project:investments",
            "decision",
            ttl_seconds=3600,
        )

        self.assertEqual(
            retrieve,
            (
                'RETRIEVE CONTEXT FOR TASK "budget \\"audit\\"\\nline" '
                "IN BRAIN investment_projects USING MODE balanced BUDGET 2048 TOKENS "
                'LIMIT 10 CANDIDATES WHERE space = project:investments AND status = "ready" '
                "REQUIRE citations REQUIRE confidence >= 0.80 REQUIRE source_trust >= 0.90 "
                "REQUIRE freshness <= 86400 SECONDS;"
            ),
        )
        self.assertEqual(
            verify,
            'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;',
        )
        self.assertEqual(
            remember,
            (
                'REMEMBER "Use conservative budget assumptions" '
                "IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;"
            ),
        )

    def test_aql_builders_reject_invalid_inputs(self) -> None:
        with self.assertRaises(ValueError):
            CortexDBClient.build_verify_fact_aql("x", "bad brain")
        with self.assertRaises(ValueError):
            CortexDBClient.build_remember_aql("x", "project:investments", "bad type")
        with self.assertRaises(ValueError):
            CortexDBClient.build_retrieve_context_aql("x", "brain", where_clause=" ")
        with self.assertRaises(ValueError):
            CortexDBClient.build_retrieve_context_aql("x", "brain", min_confidence="0")

    def test_search_path_matches_http_api_contract(self) -> None:
        path = CortexDBClient._path(
            "/v1/search",
            scope="project:investments",
            mode="keyword",
            q="solar budget",
            limit=10,
        )
        self.assertEqual(
            path,
            "/v1/search?scope=project%3Ainvestments&mode=keyword&q=solar+budget&limit=10",
        )

    def test_vector_search_path_matches_http_api_contract(self) -> None:
        path = CortexDBClient._path(
            "/v1/search",
            scope="project:investments",
            mode="vector",
            algorithm="ann",
            vector="1,2,3",
            limit=5,
        )
        self.assertEqual(
            path,
            "/v1/search?scope=project%3Ainvestments&mode=vector&algorithm=ann&vector=1%2C2%2C3&limit=5",
        )

    def test_client_with_tenant_scopes_requests(self) -> None:
        client = CortexDBClient().with_tenant("tenant:alpha")
        self.assertEqual(client._scoped("/v1/stats"), "/v1/stats?tenant=tenant%3Aalpha")
        self.assertEqual(
            client._scoped("/v1/search?scope=project%3Ainvestments"),
            "/v1/search?scope=project%3Ainvestments&tenant=tenant%3Aalpha",
        )

    def test_client_retries_database_busy_and_uses_configured_timeout(self) -> None:
        class FakeResponse:
            def __enter__(self) -> "FakeResponse":
                return self

            def __exit__(self, exc_type: object, exc: object, tb: object) -> None:
                return None

            def read(self) -> bytes:
                return b'{"ok":true}'

        class FlakyOpener:
            calls = 0
            timeouts: list[float] = []

            def open(self, request: object, timeout: float) -> FakeResponse:
                self.calls += 1
                self.timeouts.append(timeout)
                if self.calls == 1:
                    raise urllib.error.HTTPError(
                        "http://127.0.0.1:8181/v1/health",
                        503,
                        "busy",
                        {},
                        io.BytesIO(b'{"code":"database_busy","message":"busy"}'),
                    )
                return FakeResponse()

        opener = FlakyOpener()
        client = CortexDBClient(
            max_retries=1,
            retry_delay_seconds=0,
            timeout_seconds=2.5,
            _opener=opener,
        )

        self.assertEqual(client.health(), {"ok": True})
        self.assertEqual(opener.calls, 2)
        self.assertEqual(opener.timeouts, [2.5, 2.5])

    def test_client_context_manager_reuses_and_closes_opener(self) -> None:
        client = CortexDBClient()
        with client as session:
            self.assertIs(session, client)
            self.assertIsNotNone(session._opener)
        self.assertIsNone(client._opener)

    def test_typed_search_response_decodes_ann_report_contract(self) -> None:
        response = SearchResponse.from_json(
            {
                "search_mode": "vector_ann",
                "ann_report": {
                    "path": "exact_fallback",
                    "fallback_reason": "no_persisted_segments",
                    "fallback_performed": True,
                    "requested_limit": 20,
                    "allowed_candidates": 1,
                    "graph_nodes": 0,
                    "returned_candidates": 1,
                    "recall_q16": None,
                    "min_recall_q16": None,
                    "hnsw_ef_construction": 64,
                    "require_slo": True,
                    "production_safe": False,
                    "slo_violations": ["no_persisted_segments"],
                },
                "results": [
                    {
                        "cell_id": 1,
                        "score": 42,
                        "lexical_score": 0,
                        "vector_score": 42,
                        "payload": "scope=default\nstatus=ready\nhello",
                    }
                ],
            }
        )

        self.assertEqual(response.search_mode, "vector_ann")
        self.assertEqual(response.results[0].cell_id, 1)
        self.assertIsNotNone(response.ann_report)
        self.assertEqual(response.ann_report.fallback_reason, "no_persisted_segments")
        self.assertIsNone(response.ann_report.recall_q16)
        self.assertIsNone(response.ann_report.min_recall_q16)
        self.assertEqual(response.ann_report.hnsw_ef_construction, 64)
        self.assertTrue(response.ann_report.fallback_performed)
        self.assertFalse(response.ann_report.production_safe)
        self.assertEqual(response.ann_report.slo_violations, ("no_persisted_segments",))

    def test_ann_evaluation_path_matches_http_api_contract(self) -> None:
        path = CortexDBClient._path(
            "/v1/search/ann-evaluate",
            scope="project:investments",
            vector="1,2,3",
            limit=20,
        )
        self.assertEqual(
            path,
            "/v1/search/ann-evaluate?scope=project%3Ainvestments&vector=1%2C2%2C3&limit=20",
        )

    def test_typed_ann_evaluation_response_decodes_contract(self) -> None:
        response = AnnEvaluationResponse.from_json(
            {
                "available": True,
                "reason": None,
                "ann_report": {
                    "path": "hnsw_graph",
                    "fallback_reason": None,
                    "fallback_performed": False,
                    "requested_limit": 20,
                    "allowed_candidates": 2,
                    "graph_nodes": 2,
                    "returned_candidates": 2,
                    "recall_q16": 65535,
                    "min_recall_q16": 65535,
                    "hnsw_ef_construction": 128,
                    "require_slo": True,
                    "production_safe": True,
                    "slo_violations": [],
                },
                "exact_top_k": [2, 1],
                "ann_top_k": [2, 1],
                "overlap_count": 2,
                "recall_q16": 65535,
            }
        )

        self.assertTrue(response.available)
        self.assertEqual(response.exact_top_k, (2, 1))
        self.assertIsNotNone(response.ann_report)
        self.assertEqual(response.ann_report.path, "hnsw_graph")
        self.assertEqual(response.ann_report.recall_q16, 65535)
        self.assertEqual(response.ann_report.min_recall_q16, 65535)
        self.assertEqual(response.ann_report.hnsw_ef_construction, 128)
        self.assertTrue(response.ann_report.require_slo)
        self.assertTrue(response.ann_report.production_safe)

    def test_error_response_decodes_full_core_alpha_taxonomy(self) -> None:
        codes = (
            "not_found",
            "bad_request",
            "unauthorized",
            "forbidden",
            "payload_too_large",
            "rate_limited",
            "service_unavailable",
            "internal",
            "invalid_aql",
            "permission_denied",
            "database_busy",
            "storage_corruption",
            "invalid_tenant",
        )

        for code in codes:
            error = CortexDBError.from_response(
                400,
                f'{{"code":"{code}","error":"{code}","message":"message"}}',
            )
            self.assertEqual(error.code, code)
            self.assertEqual(error.status, 400)
            self.assertEqual(
                error.body,
                f'{{"code":"{code}","error":"{code}","message":"message"}}',
            )

    def test_ingest_path_matches_http_api_contract(self) -> None:
        path = CortexDBClient._path(
            "/v1/ingest/text",
            scope="project:investments",
            source="python sdk",
        )
        self.assertEqual(
            path,
            "/v1/ingest/text?scope=project%3Ainvestments&source=python+sdk",
        )

    def test_grounded_answer_helper_builds_context_verify_and_citations(self) -> None:
        class FakeClient(CortexDBClient):
            context_calls: list[tuple[str, str]] = []
            verify_calls: list[tuple[str, str]] = []

            def context_response(self, scope: str, statement: str) -> ContextPackResponse:
                type(self).context_calls.append((scope, statement))
                return ContextPackResponse.from_json(
                    {
                        "schema_version": "context_pack.v1",
                        "token_budget_tokens": 256,
                        "estimated_tokens": 40,
                        "truncated": False,
                        "citations_required": True,
                        "cells": [
                            {
                                "cell_id": 7,
                                "estimated_tokens": 40,
                                "citation": "doc://project-risk#p1",
                                "payload_text": "The migration blocker is the audit export dependency.",
                                "explain": None,
                                "source_ref": None,
                            }
                        ],
                        "anomalies": [],
                    }
                )

            def verify_response(self, scope: str, statement: str) -> VerificationReportResponse:
                type(self).verify_calls.append((scope, statement))
                return VerificationReportResponse.from_json(
                    {
                        "fact": "The migration blocker is the audit export dependency.",
                        "status": "supported",
                        "verdict": "supported",
                        "confidence_q16": 60000,
                        "evidence": [],
                        "contradicting_evidence": [],
                        "guards": [],
                        "supporting": [],
                        "contradicting": [],
                        "numeric_conflicts": [],
                    }
                )

        response = FakeClient().answer_with_grounded_context(
            "project:alpha",
            "default",
            "migration blocker",
            lambda context: "The migration blocker is the audit export dependency.",
            mode="balanced",
            limit_candidates=5,
            where_clause='space = project:alpha AND status = "ready"',
            require_citations=True,
            reject_unsupported=True,
        )
        assert response.verification is not None
        assert response.verification.confidence_q16 == 60000

        self.assertEqual(response.citations, ("doc://project-risk#p1",))
        self.assertEqual(response.used_context_cell_ids, (7,))
        self.assertTrue(response.grounding.answer_supported)
        self.assertFalse(response.rejected)
        self.assertEqual(response.verification.status if response.verification else None, "supported")
        self.assertIn("LIMIT 5 CANDIDATES", response.retrieve_statement)
        self.assertEqual(len(FakeClient.context_calls), 1)
        self.assertEqual(len(FakeClient.verify_calls), 1)


if __name__ == "__main__":
    unittest.main()
