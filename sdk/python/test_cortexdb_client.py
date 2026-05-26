import unittest

from cortexdb_client import AnnEvaluationResponse, CortexDBClient, SearchResponse


class CortexDBClientPathTests(unittest.TestCase):
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

    def test_typed_search_response_decodes_ann_report_contract(self) -> None:
        response = SearchResponse.from_json(
            {
                "search_mode": "vector_ann",
                "ann_report": {
                    "path": "exact_fallback",
                    "fallback_reason": "no_persisted_segments",
                    "requested_limit": 20,
                    "allowed_candidates": 1,
                    "graph_nodes": 0,
                    "returned_candidates": 1,
                    "recall_q16": None,
                    "min_recall_q16": None,
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
                    "requested_limit": 20,
                    "allowed_candidates": 2,
                    "graph_nodes": 2,
                    "returned_candidates": 2,
                    "recall_q16": 65535,
                    "min_recall_q16": 65535,
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


if __name__ == "__main__":
    unittest.main()
