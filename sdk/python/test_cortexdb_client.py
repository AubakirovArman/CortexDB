import unittest

from cortexdb_client import CortexDBClient


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


if __name__ == "__main__":
    unittest.main()
