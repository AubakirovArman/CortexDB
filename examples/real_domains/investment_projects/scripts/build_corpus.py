#!/usr/bin/env python3
"""Build the investment_projects real-domain ANN corpus seed."""

from __future__ import annotations

import argparse
import json
import re
import textwrap
import urllib.request
from pathlib import Path
from typing import Any


WB_URL = (
    "https://search.worldbank.org/api/v2/projects?format=json"
    "&countrycode_exact=KZ&rows={rows}"
    "&fl=project_name,countryname,boardapprovaldate,closingdate,totalamt,"
    "sector1,sector2,sector3,theme1,theme2,projectstatusdisplay,countryshortname,url"
)

MANUAL_DOCS = [
    {
        "doc_id": "edb_bakad_001",
        "source": "edb",
        "country": "Kazakhstan",
        "title": "Big Almaty Ring Road BAKAD",
        "sector": "transport infrastructure",
        "url": "https://eabr.org/en/projects/in-process/construction-and-operation-of-the-big-almaty-ring-road-bakad/",
        "tags": ["bakad", "almaty", "ring road", "ppp", "transport", "edb"],
        "facts": [
            "EDB describes BAKAD as a 66 km ring road around Almaty and a large PPP transport project.",
            "The project connects Almaty with regional and Europe-Western China transport corridors.",
            "The source records EDB, EBRD, and IsDB participation in the lender group.",
        ],
    },
    {
        "doc_id": "edb_almaty_airport_001",
        "source": "edb",
        "country": "Kazakhstan",
        "title": "Expansion and modernisation of Almaty International Airport",
        "sector": "transport infrastructure",
        "url": "https://eabr.org/en/projects/in-process/expansion-and-modernisation-of-almaty-international-airport/",
        "tags": ["almaty", "airport", "terminal", "transport", "edb", "ifc", "ebrd"],
        "facts": [
            "The airport project covers a new terminal and modernisation of the existing airport building.",
            "The project aims to remove capacity constraints and improve passenger and freight services.",
            "The source lists a lender syndicate involving EBRD, IFC, DEG, and EDB.",
        ],
    },
    {
        "doc_id": "ebrd_kaz_renewables_framework_001",
        "source": "ebrd",
        "country": "Kazakhstan",
        "title": "Kazakhstan renewable energy transition and grid integration",
        "sector": "energy",
        "url": "https://www.ebrd.com/home/what-we-do/impact-management/impact-case-studies/supporting-the-renewable-energy-transition-in-kazakhstan.html",
        "tags": ["renewable", "energy", "grid", "qajet", "mirny", "battery", "ebrd"],
        "facts": [
            "EBRD describes a Kazakhstan renewables framework supporting generation, grid integration, and policy reform.",
            "The transition narrative includes competitive auctions, PPA improvements, and private investment mobilisation.",
            "The source references the Mirny wind project with battery storage as a landmark renewable project.",
        ],
    },
    {
        "doc_id": "ebrd_kegoc_west_zone_001",
        "source": "ebrd",
        "country": "Kazakhstan",
        "title": "KEGOC integration of the West Zone",
        "sector": "energy",
        "url": "https://www.ebrd.com/content/dam/ebrd_dxp/documents/project/55284/kegoc-integration-of-the-west-zone.pdf",
        "tags": ["kegoc", "west zone", "grid", "renewable", "battery", "ebrd"],
        "facts": [
            "The KEGOC project is represented as grid infrastructure supporting renewable energy integration.",
            "The benchmark uses it for queries about power transmission, grid balancing, and renewable absorption.",
            "The project belongs to the energy infrastructure domain rather than oil and gas extraction.",
        ],
    },
    {
        "doc_id": "reuters_mirny_reference_001",
        "source": "reuters_reference",
        "country": "Kazakhstan",
        "title": "Mirny wind farm investment reference",
        "sector": "renewable energy",
        "url": "https://www.reuters.com/business/energy/totalenergies-invest-12-billion-power-project-kazakhstan-2026-04-24/",
        "tags": ["mirny", "totalenergies", "wind", "battery", "kazmunaygas", "samruk"],
        "facts": [
            "This metadata-only reference is used for queries about the Mirny wind farm, TotalEnergies, and battery storage.",
            "The corpus does not copy the news article body; it stores only benchmark metadata and short factual tags.",
        ],
    },
    {
        "doc_id": "reuters_tengiz_reference_001",
        "source": "reuters_reference",
        "country": "Kazakhstan",
        "title": "Tengiz expansion investment reference",
        "sector": "oil and gas",
        "url": "https://www.reuters.com/markets/commodities/chevron-starts-48-billion-kazakh-oilfield-expansion-2025-01-24/",
        "tags": ["tengiz", "chevron", "oilfield", "expansion", "production"],
        "facts": [
            "This metadata-only reference is used for project-specific queries about Tengiz and Chevron.",
            "The corpus does not copy the news article body; it stores only benchmark metadata and short factual tags.",
        ],
    },
]

QUERIES = [
    ("q001", "Kazakhstan airport infrastructure financing project", "find_project_by_sector_country", ["airport"]),
    ("q002", "Kazakhstan renewable energy investment project", "find_project_by_sector", ["renewable", "energy"]),
    ("q003", "Kazakhstan transport corridor project", "find_project_by_sector", ["transport", "corridor"]),
    ("q004", "Kazakhstan water infrastructure development project", "find_project_by_sector", ["water"]),
    ("q005", "Kazakhstan manufacturing investment project", "find_project_by_sector", ["manufacturing", "industry"]),
    ("q006", "Kazakhstan agriculture investment project", "find_project_by_sector", ["agriculture", "livestock"]),
    ("q007", "Kazakhstan green energy financing", "find_project_by_impact", ["green", "renewable", "energy"]),
    ("q008", "Kazakhstan logistics infrastructure project", "find_project_by_sector", ["logistics", "transport"]),
    ("q009", "Almaty airport reconstruction financing", "find_project_by_name", ["almaty", "airport"]),
    ("q010", "Big Almaty Ring Road project", "find_project_by_name", ["bakad", "ring road"]),
    ("q011", "Tengiz oilfield expansion investment", "find_project_by_name", ["tengiz", "oilfield"]),
    ("q012", "Mirny wind farm Kazakhstan investment", "find_project_by_name", ["mirny", "wind"]),
    ("q013", "Solar power project Kazakhstan financing", "find_project_by_sector", ["solar", "renewable"]),
    ("q014", "transport infrastructure project in Almaty", "find_project_by_region", ["almaty", "transport"]),
    ("q015", "Kazakhstan airport terminal project", "find_project_by_asset", ["airport", "terminal"]),
    ("q016", "Kazakhstan road construction PPP project", "find_project_by_finance_model", ["road", "ppp"]),
    ("q017", "project with investment over one billion dollars in Kazakhstan", "find_project_by_amount", ["billion", "large"]),
    ("q018", "Kazakhstan project financed by international development bank", "find_project_by_financier", ["world bank", "edb", "ebrd"]),
    ("q019", "infrastructure project with external financing", "find_project_by_financier", ["infrastructure", "financing"]),
    ("q020", "renewable energy project with battery storage", "find_project_by_asset", ["battery", "renewable"]),
    ("q021", "large oilfield expansion Kazakhstan", "find_project_by_sector", ["oilfield", "tengiz"]),
    ("q022", "public private partnership Kazakhstan transport", "find_project_by_finance_model", ["ppp", "transport"]),
    ("q023", "TotalEnergies Kazakhstan wind farm project", "find_project_by_company", ["totalenergies", "wind"]),
    ("q024", "Chevron Tengiz expansion project", "find_project_by_company", ["chevron", "tengiz"]),
    ("q025", "KazMunayGas renewable energy partnership", "find_project_by_company", ["kazmunaygas", "renewable"]),
    ("q026", "Samruk Energy wind project", "find_project_by_company", ["samruk", "wind"]),
    ("q027", "EDB Kazakhstan infrastructure project", "find_project_by_financier", ["edb", "infrastructure"]),
    ("q028", "IFC Kazakhstan private sector project", "find_project_by_financier", ["ifc", "private"]),
    ("q029", "project budget 1.2 billion Kazakhstan", "verify_project_amount", ["1.2", "billion", "mirny"]),
    ("q030", "project budget 1.4 billion Kazakhstan", "verify_project_amount", ["1.4", "billion"]),
    ("q031", "project expected completion 2029 Kazakhstan", "verify_project_date", ["2029", "mirny"]),
    ("q032", "project with 600 MWh battery system", "verify_project_asset", ["600", "mwh", "battery"]),
    ("q033", "Kazakhstan project increasing renewable energy share", "find_project_by_impact", ["renewable", "share"]),
    ("q034", "project expected to increase oil production", "find_project_by_impact", ["oil", "production"]),
    ("q035", "which projects support Kazakhstan green transition", "semantic_green_transition", ["green", "transition", "renewable"]),
    ("q036", "which investment projects improve transport infrastructure", "semantic_transport", ["transport", "infrastructure"]),
    ("q037", "projects related to airport modernization in Kazakhstan", "semantic_airport", ["airport", "modernisation", "modernization"]),
    ("q038", "investment projects involving state-owned Kazakh companies", "semantic_company", ["samruk", "kazmunaygas", "state"]),
    ("q039", "projects with environmental and social disclosure", "semantic_esg", ["environmental", "social", "disclosure"]),
    ("q040", "projects financed by multilateral development banks", "semantic_financier", ["world bank", "edb", "ebrd", "ifc"]),
]


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")[:80]


def compact(value: Any) -> str:
    if isinstance(value, dict):
        for key in ("Name", "name", "sector", "theme"):
            named = value.get(key)
            if named:
                return str(named)
        return " ".join(str(v) for v in value.values() if v)
    if value is None:
        return ""
    cleaned = re.sub(r"!\$!\d+", "", str(value))
    cleaned = re.sub(r"\s+", " ", cleaned)
    return cleaned.strip(" !$")


def fetch_world_bank(rows: int) -> list[dict]:
    with urllib.request.urlopen(WB_URL.format(rows=rows), timeout=30) as response:
        data = json.load(response)
    docs = []
    for project_id, row in sorted(data.get("projects", {}).items()):
        title = compact(row.get("project_name"))
        if not title:
            continue
        sector = compact(row.get("sector1") or row.get("theme1")) or "development policy"
        doc_id = f"wb_{project_id.lower()}"
        url = compact(row.get("url")) or f"https://projects.worldbank.org/en/projects-operations/project-detail/{project_id}"
        amount = compact(row.get("totalamt")) or "not specified"
        status = compact(row.get("projectstatusdisplay")) or "unknown"
        country = compact(row.get("countryshortname") or row.get("countryname") or "Kazakhstan")
        tags = [title, sector, compact(row.get("theme1")), compact(row.get("theme2")), status, "world bank"]
        facts = [
            f"World Bank project {project_id} is titled {title}.",
            f"The project country is {country}; status is {status}; total commitment is {amount}.",
            f"The project is tagged for benchmark retrieval under sector {sector}.",
            "This generated benchmark note stores structured project metadata, not copied source document bodies.",
        ]
        docs.append({
            "doc_id": doc_id,
            "source": "world_bank",
            "country": country,
            "title": title,
            "sector": sector,
            "url": url,
            "tags": [tag.lower() for tag in tags if tag],
            "facts": facts,
        })
    return docs


def document_text(doc: dict) -> str:
    tags = ", ".join(doc["tags"])
    paragraphs = [
        f"{doc['title']} is a {doc['sector']} project record for {doc['country']}.",
        f"Source: {doc['source']}. URL: {doc['url']}.",
        f"Retrieval tags: {tags}.",
        *doc["facts"],
        "Analyst use cases include country-sector lookup, project-specific retrieval, financier search, amount verification, status review, and evidence-aware context packing.",
        f"Country filter notes: this row should be retrieved for Kazakhstan investment-project questions and for Central Asia project discovery when the query mentions {doc['sector']}.",
        f"Entity and sector notes: the title, source, sector, country, URL, and retrieval tags are repeated in structured form so embedding tests can evaluate project-name, financier, sector, and impact matching without relying on copied source prose.",
        "Ground-truth notes: chunks from this document are relevant when a query asks for the named project, the same sector, the same country, the listed financier/source, or one of the explicit retrieval tags.",
        "This corpus row is designed for CortexDB ANN/HNSW embedding evaluation in the investment_projects domain for Kazakhstan and Central Asia.",
    ]
    return "\n\n".join(textwrap.fill(p, width=100) for p in paragraphs)


def chunk_text(doc: dict, size: int, overlap: int, min_size: int) -> list[dict]:
    text = doc["text"]
    chunks = []
    start = 0
    index = 1
    while start < len(text):
        part = text[start:start + size].strip()
        if len(part) >= min_size:
            chunks.append({
                "chunk_id": f"{doc['doc_id']}_c{index:03d}",
                "doc_id": doc["doc_id"],
                "source": doc["source"],
                "country": doc["country"],
                "sector": doc["sector"],
                "title": doc["title"],
                "text": part,
                "payload": part,
            })
            index += 1
        if start + size >= len(text):
            break
        start += size - overlap
    return chunks


def build_queries(chunks_by_doc: dict[str, list[str]], docs: list[dict]) -> tuple[list[dict], list[dict]]:
    searchable = {doc["doc_id"]: " ".join([doc["title"], doc["sector"], " ".join(doc["tags"]), doc["text"]]).lower() for doc in docs}
    queries = []
    ground_truth = []
    for query_id, query, intent, terms in QUERIES:
        matches = []
        for doc_id, haystack in searchable.items():
            if any(term.lower() in haystack for term in terms):
                matches.append(doc_id)
        if not matches:
            matches = [docs[0]["doc_id"]]
        matches = matches[:5]
        chunk_ids = [chunk for doc_id in matches for chunk in chunks_by_doc.get(doc_id, [])[:2]]
        queries.append({"query_id": query_id, "name": query_id, "query": query, "text": query, "intent": intent, "limit": 5})
        ground_truth.append({"query_id": query_id, "name": query_id, "relevant_doc_ids": matches, "relevant_chunk_ids": chunk_ids})
    return queries, ground_truth


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--world-bank-rows", type=int, default=50)
    parser.add_argument("--chunk-size", type=int, default=800)
    parser.add_argument("--overlap", type=int, default=120)
    parser.add_argument("--min-chunk-size", type=int, default=100)
    args = parser.parse_args()

    docs = fetch_world_bank(args.world_bank_rows) + MANUAL_DOCS
    materialized = []
    for doc in docs:
        row = {key: doc[key] for key in ("doc_id", "source", "country", "title", "sector", "url")}
        row["text"] = document_text(doc)
        row["payload"] = row["text"]
        materialized.append(row)
    chunks = [chunk for doc in materialized for chunk in chunk_text(doc, args.chunk_size, args.overlap, args.min_chunk_size)]
    chunks_by_doc: dict[str, list[str]] = {}
    for chunk in chunks:
        chunks_by_doc.setdefault(chunk["doc_id"], []).append(chunk["chunk_id"])
    queries, truth = build_queries(chunks_by_doc, [{**doc, "text": row["text"]} for doc, row in zip(docs, materialized)])
    write_jsonl(args.root / "corpus" / "documents.jsonl", materialized)
    write_jsonl(args.root / "corpus" / "chunks.jsonl", chunks)
    write_jsonl(args.root / "queries" / "queries.jsonl", queries)
    write_jsonl(args.root / "queries" / "ground_truth.jsonl", truth)
    print(json.dumps({"documents": len(materialized), "chunks": len(chunks), "queries": len(queries)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
