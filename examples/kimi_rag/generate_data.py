import json
import random
import sys
import urllib.request

def generate_mock_data():
    # Database 1: financial_records
    financial_cells = []
    projects = [f"Project_{chr(65+i)}" for i in range(10)] + ["Solar Plant"]
    
    # Table 1: revenue (50 rows)
    for i in range(50):
        proj = random.choice(projects)
        year = random.randint(2024, 2026)
        val = random.randint(100, 900)
        financial_cells.append({
            "scope": "financial_records",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=project:investments\nstatus=ready\ntype=fact\nproject={proj}\nmetric=revenue\nvalue={val}000000\ncurrency=USD\n\n{proj} annual revenue for fiscal year {year} was recorded at {val}M USD based on ledger audit."
        })
        
    # Table 2: expenses (50 rows)
    for i in range(50):
        proj = random.choice(projects)
        year = random.randint(2024, 2026)
        val = random.randint(50, 400)
        financial_cells.append({
            "scope": "financial_records",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=project:investments\nstatus=ready\ntype=fact\nproject={proj}\nmetric=expense\nvalue={val}000000\ncurrency=USD\n\n{proj} operating expenses in fiscal year {year} amounted to {val}M USD according to ledger audit."
        })

    # Table 3: budgets (50 rows) - Including our Solar Plant conflict!
    financial_cells.append({
        "scope": "financial_records",
        "status": "ready",
        "type": "fact",
        "payload": "scope=project:investments\nstatus=ready\ntype=fact\nproject=Solar Plant\nmetric=budget\nvalue=1.2\ncurrency=KZT\n\nSolar Plant approved budget in Q1 is 1.2B KZT."
    })
    financial_cells.append({
        "scope": "financial_records",
        "status": "ready",
        "type": "fact",
        "payload": "scope=project:investments\nstatus=ready\ntype=fact\nproject=Solar Plant\nmetric=budget\nvalue=1400000000\ncurrency=KZT\n\nSolar Plant adjusted budget in Q2 is 1.4B KZT."
    })
    for i in range(48):
        proj = random.choice(projects)
        val = random.choice([1.1, 1.3, 1.5, 2.0, 2.4])
        financial_cells.append({
            "scope": "financial_records",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=project:investments\nstatus=ready\ntype=fact\nproject={proj}\nmetric=budget\nvalue={val}\ncurrency=KZT\n\n{proj} allocated project development budget is set to {val}B KZT."
        })

    # Table 4: investments (50 rows)
    for i in range(50):
        proj = random.choice(projects)
        val = random.randint(10, 150)
        financial_cells.append({
            "scope": "financial_records",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=project:investments\nstatus=ready\ntype=fact\nproject={proj}\nmetric=investment\nvalue={val}000000\ncurrency=EUR\n\n{proj} capital expenditure investment received total of {val}M EUR from external venture funds."
        })

    # Database 2: legal_compliance
    legal_cells = []
    entities = [f"Entity_{chr(65+i)}" for i in range(10)]
    
    # Table 1: contracts (50 rows)
    for i in range(50):
        ent = random.choice(entities)
        val = random.randint(10, 90)
        legal_cells.append({
            "scope": "legal_compliance",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=legal:contracts\nstatus=ready\ntype=fact\nproject={ent}\nmetric=contract_value\nvalue={val}000000\ncurrency=USD\n\n{ent} legally signed master service agreement contract is valued at {val}M USD with standard SLA terms."
        })

    # Table 2: regulations (50 rows)
    for i in range(50):
        ent = random.choice(entities)
        code = random.randint(100, 999)
        legal_cells.append({
            "scope": "legal_compliance",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=legal:contracts\nstatus=ready\ntype=fact\nproject={ent}\nmetric=compliance_code\nvalue={code}\ncurrency=REG\n\n{ent} operation complies with international audit regulation code REG-{code} regarding data sovereignty."
        })

    # Table 3: penalties (50 rows)
    for i in range(50):
        ent = random.choice(entities)
        val = random.randint(5, 50)
        legal_cells.append({
            "scope": "legal_compliance",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=legal:contracts\nstatus=ready\ntype=fact\nproject={ent}\nmetric=penalty\nvalue={val}0000\ncurrency=EUR\n\n{ent} late delivery SLA penalty is capped at {val}K EUR per business day of non-sovereignty compliance."
        })

    # Table 4: signatories (50 rows)
    names = ["John Smith", "Alice Johnson", "David Miller", "Emma Wilson", "Robert Taylor"]
    for i in range(50):
        ent = random.choice(entities)
        name = random.choice(names)
        legal_cells.append({
            "scope": "legal_compliance",
            "status": "ready",
            "type": "fact",
            "payload": f"scope=legal:contracts\nstatus=ready\ntype=fact\nproject={ent}\nmetric=authorized_signatory\nvalue=1\ncurrency=AUTH\n\n{ent} authorized corporate signatory is {name} under standard executive board resolutions."
        })

    return financial_cells + legal_cells

def ingest_data(cells):
    print(f"🔄 Ingesting {len(cells)} knowledge cells into CortexDB...")
    for i, cell in enumerate(cells):
        url = f"http://127.0.0.1:8090/put?cell_id={i+1}&tenant={cell['scope']}"
        req = urllib.request.Request(
            url,
            data=cell["payload"].encode("utf-8"),
            headers={"Content-Type": "text/plain"},
            method="POST"
        )
        try:
            with urllib.request.urlopen(req) as res:
                res.read()
        except Exception as e:
            print(f"❌ Ingestion failed at cell {i+1}: {e}")
            sys.exit(1)
    print("✅ All 400 rows of relational mock data successfully ingested into CortexDB realms!")

if __name__ == "__main__":
    cells = generate_mock_data()
    ingest_data(cells)
