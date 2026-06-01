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

    context = client.context_response(
        "default",
        'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default LIMIT 10 CANDIDATES;',
    )
    print(f"context_tokens={context.estimated_tokens}")


if __name__ == "__main__":
    main()
