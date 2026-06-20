# Runtime Security Audit Residual Closure Note

Baseline commit for this note: `84741bb6803bccdd4eb6176d8af71ab0aee34d2a`.

## Closed in current tree

- Runtime session artifacts under `.vac/session/**` are absent from the current tree and ignored/package-filtered recursively.
- Route-check bypass for protected autopilot routes is loopback-only. Public, unspecified, or LAN binds fail closed with `no_auth_requires_loopback_bind`; loopback IPv4 and IPv6 remain allowed for local development.
- Remote read/list/grep paths no longer build remote shell command strings. Remote glob and grep now use SFTP traversal/read plus local glob and regex matching. The explicit remote command execution tool remains separate and unchanged.
- Remote session directory creation no longer shells out to `mkdir -p`; it uses SFTP directory creation.
- SSH `known_hosts` verification remains fail-closed and now covers plain hosts, comma-separated hosts, bracketed ports, OpenSSH hashed hosts, and `@revoked` hard rejection. `@cert-authority` entries are parsed but do not authorize raw host keys.
- Container host credential mounts remain opt-in and now emit runtime warnings when enabled.

## Operational residuals not closed by code patch

- Historical secret/credential exposure cannot be closed by deleting files from the current tree. Any token/key/material that was ever committed under `.vac/session/**` must be revoked and replaced by the owning provider/account.
- Git history purge is intentionally not performed here. Removing `.vac/session/**` from reachable history requires a coordinated destructive rewrite, `--force-with-lease`, and clone remediation.
- Full workspace validation is only closed when the named commands are actually run and pass. Until then, status wording must remain `NotEvaluated` for gates not executed locally or in CI.

## Operator checklist for remaining P0 operations

1. Inventory affected path names from history without printing raw secret values.
2. Revoke old provider/local/gateway/broker tokens and issue replacements into a non-repo secret store.
3. Run a history rewrite for `.vac/session/**` only after branch freeze and explicit operator approval.
4. Re-clone or hard-reset old clones after remote rewrite; treat stale local clones and backups as still containing compromised material.
