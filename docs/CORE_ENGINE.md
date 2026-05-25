# Core Engine

`cortex-engine` is the first facade over the lower layers.

It owns:

- database directory creation
- WAL path selection (`db.aclog`)
- WAL replay during open
- WAL writer startup
- MemTable updates after successful WAL append
- snapshot reads from the current commit sequence

It does not execute AQL yet. AQL still compiles to bound plans and bitmap
programs; connecting those plans to persisted cells is a later milestone.
