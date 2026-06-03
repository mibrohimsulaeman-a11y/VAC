# VAC npm release scripts

Use the staging helper to generate npm tarballs for VAC packages.

Example:

```bash
./scripts/stage_npm_packages.py   --release-version 0.6.0   --package vac   --package vac-responses-api-proxy   --package vac-sdk
```

The script names are still inherited from the VAC base, but product package output must be VAC-facing.

When the CLI package is staged, it should build the lightweight `@vac/cli` package plus platform-native `@vac/cli-*` variants.

Do not reintroduce `@vastar/vac` package names into VAC product packaging unless explicitly documenting an upstream compatibility shim.
