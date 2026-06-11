# vac-remote-service

Optional remote-service adapter for VAC. This crate replaces legacy server-bound assumptions with a VAC integration boundary.

Default runtime remains local control plane. Remote calls must be explicit, policy-governed, and recorded as evidence when they affect runtime authority or release state.
