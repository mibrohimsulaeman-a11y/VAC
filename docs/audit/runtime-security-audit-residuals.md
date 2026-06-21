# Runtime Security Audit Residual Closure Note

Baseline commit for this note before local history rewrite: `84741bb6803bccdd4eb6176d8af71ab0aee34d2a`. Local history rewrite was later applied in this clone; verify with `git log --all --name-only -- '**/.vac/session/**'` before publishing.

## Closed in current tree

- Runtime session artifacts under `.vac/session/**` are absent from the current tree and ignored/package-filtered recursively.
- Local reachable history in this clone has been rewritten with `git-filter-repo` to remove the known nested `.vac/session/**` roots; local verification showed no remaining `.vac/session/**` path names in `git log --all`.
- Route-check bypass for protected autopilot routes is loopback-only. Public, unspecified, or LAN binds fail closed with `no_auth_requires_loopback_bind`; loopback IPv4 and IPv6 remain allowed for local development.
- Remote read/list/grep paths no longer build remote shell command strings. Remote glob and grep now use SFTP traversal/read plus local glob and regex matching. The explicit remote command execution tool remains separate and unchanged.
- Remote session directory creation no longer shells out to `mkdir -p`; it uses SFTP directory creation.
- SSH `known_hosts` verification remains fail-closed and now covers plain hosts, comma-separated hosts, bracketed ports, OpenSSH hashed hosts, and `@revoked` hard rejection. `@cert-authority` entries are parsed but do not authorize raw host keys.
- Container host credential mounts remain opt-in and now emit runtime warnings when enabled.

## Operational residuals not closed by code patch

- Historical secret/credential exposure cannot be closed by deleting files from the current tree. Any token/key/material that was ever committed under `.vac/session/**` must be revoked and replaced by the owning provider/account.
- Remote Git history is not remediated until the rewritten local history is force-pushed with lease and all old clones/backups are remediated.
- Full workspace validation is only closed when the named commands are actually run and pass. Until then, status wording must remain `NotEvaluated` for gates not executed locally or in CI.

## Operator checklist for remaining P0 operations

1. Inventory affected path names from history without printing raw secret values.
2. Revoke old provider/local/gateway/broker tokens and issue replacements into a non-repo secret store.
3. Force-push the rewritten history only after branch freeze and explicit operator approval.
4. Re-clone or hard-reset old clones after remote rewrite; treat stale local clones and backups as still containing compromised material.
