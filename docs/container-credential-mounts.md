# Container Credential Mounts

VAC agent containers start with a minimal mount profile. Host cloud directories, kube config, and SSH material are not mounted by default because the L1 cooperative sandbox is not a credential boundary.

Enable host material only with explicit operator intent:

| Provider | Opt-in flag |
| --- | --- |
| AWS | `VAC_AGENT_MOUNT_AWS=1` |
| GCP | `VAC_AGENT_MOUNT_GCP=1` |
| Azure | `VAC_AGENT_MOUNT_AZURE=1` |
| DigitalOcean | `VAC_AGENT_MOUNT_DIGITALOCEAN=1` |
| Kubernetes | `VAC_AGENT_MOUNT_KUBE=1` |
| SSH | `VAC_AGENT_MOUNT_SSH=1` |

Accepted true values are `1` and `true` case-insensitively. Other values do not enable a mount. When a flag is enabled the runtime emits a warning that a credential mount is active; the warning names the flag/provider and does not print raw secret values.

Use the default minimal profile for ordinary local work. Use opt-in mounts only for tasks that genuinely need host cloud, kube, or SSH state, and revoke or rotate any exposed material if the workspace boundary is later suspected to be compromised.
