# Failure Scenarios

## WAL Tail

- Strict recovery rejects corrupted WAL records.
- Best-effort recovery stops at the last safe offset.
- Partial WAL tails are truncated before the next writer starts.

## Checkpoint Files

- `.acs`, `.acb`, `.aci`, and `.acm` files include CRC32C footers.
- Corrupted live files fail validation.
- Temp files with known checkpoint/index extensions are removed during open.
- Atomic writes use unique temp filenames of the form
  `<target>.tmp.<pid>.<counter>`.

## Manifest Consistency

- Duplicate live segment ids fail validation.
- Live and retired segment id overlap fails validation.
- Manifest `checkpoint_seq` must not lag behind a live segment checkpoint.
- Candidate id `0` fails validation.
- A candidate id may be reused by an update only if it still maps to the same
  `CellId`.

## Locking

- `Database::open` creates `db.lock`.
- The lock file records `format=cortexdb-lock-v1`, process id, creation Unix
  timestamp, and database root for operator inspection.
- A second open fails while the first database handle is alive.
- `Database::close` and `Drop` release the lock after shutting down the writer.
- If a process dies, stale `db.lock` cleanup is explicit through
  `cortexdb unlock <path> --force` or `StaleLockPolicy::Break`.
