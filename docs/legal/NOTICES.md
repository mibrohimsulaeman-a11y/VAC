# Legal notices

This file is the home for legal notices, third-party attributions, and license references that apply to the VAC repository.

## Separation rule

Legal notices are not product identity. Keep VAC product identity in the root README, `AGENTS.md`, and `.vac/registry/product.yaml`; keep attribution and license text here or in source-specific license files.

## Current status

- Product identity: VAC
- Product command: `vac`
- Notice scope: repository-level legal and third-party attribution material

When new dependencies, bundled source material, or donor excerpts require attribution, add the notice here without changing the VAC product name, command, or root architecture contract.

## Third-party attributions

See `THIRD_PARTY_NOTICES.md` for repository-level third-party notices. Dependency attribution generation remains `NotEvaluated` until the legal gate is executed.

## Source-specific licenses

Donor source under `donor/vac/` retains its upstream license headers. When porting donor code into the root product, preserve the original attribution comment in the ported file or record the donor-origin attribution here.


## Dependency attribution status

`cargo about` / dependency attribution generation is currently `NotEvaluated` in sandbox artifacts unless explicitly executed and attached.
