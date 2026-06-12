# Incident Playbooks Evidence

Focused gate:

```bash
make incident-playbooks-check
```

Primary artifact:

```text
target/incident-playbooks/report.json
```

## What It Proves

The gate verifies that [`INCIDENT_PLAYBOOKS.md`](INCIDENT_PLAYBOOKS.md)
contains operator-ready playbooks for:

- corrupted storage;
- actor busy / queue pressure;
- backup failed / stale backup evidence;
- auth failure spike;
- tenant issue.

Each playbook must include trigger examples, triage commands, containment,
recovery steps, and exit criteria. The gate also checks for the operational
commands that operators need during local single-node incidents, including
`cortexdb validate`, `cortexdb repair`, `backup-drill`, `/v1/metrics`, audit
review, and tenant recovery gates.

## Boundary

This evidence covers local single-node incident response. It does not claim:

- managed paging or incident routing;
- production multi-node failover;
- enterprise SOC workflow integration;
- legal or compliance incident certification.
