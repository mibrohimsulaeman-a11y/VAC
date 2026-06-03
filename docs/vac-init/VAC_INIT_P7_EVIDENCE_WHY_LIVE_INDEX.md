# VAC-Init P7 Evidence Writer and vac why Live Index

This slice adds a live evidence and trajectory writer that can create evidence records and `.vac/registry/trajectory/index.yaml` entries consumed by `vac why`.

The writer rejects raw/private chain-of-thought markers in safe rationale summaries.
