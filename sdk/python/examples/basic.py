"""Basic CortexDB Python SDK usage.

Run with a local server:

    python sdk/python/examples/basic.py

The example expects cortex-server at http://127.0.0.1:8181.
"""

from cortexdb_client import CortexDBClient


def main() -> None:
    client = CortexDBClient("http://127.0.0.1:8181")

    health = client.health_response()
    print(f"server_version={health.server_version}")

    put = client.put_cell(
        1,
        "scope=default\nstatus=ready\ntype=fact\nsource=python-sdk\n\nhello world",
    )
    print(f"put={put}")

    lookup = client.get_cell(1)
    print(f"cell={lookup}")

    search = client.search_response("default", "hello")
    print(f"search_results={len(search.results)}")

    aql = client.build_retrieve_context_aql("hello", "default", limit_candidates=10)
    context = client.context_response(
        "default",
        aql,
    )
    print(f"context_tokens={context.estimated_tokens}")


if __name__ == "__main__":
    main()
