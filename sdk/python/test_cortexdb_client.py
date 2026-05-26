import unittest

from cortexdb_client import CortexDBClient, SearchResponse


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
