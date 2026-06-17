# VAC providers

`vac-provider-core` is the only active provider implementation crate in this workspace. It owns provider request/response normalization, streaming conversion, and provider-specific adapter code for Anthropic, OpenAI, Google/Gemini, Bedrock, OpenRouter, Copilot, and local/VAC routing.

Per-provider crates such as `vac-provider-anthropic`, `vac-provider-bedrock`, `vac-provider-local`, and `vac-provider-openai` are intentionally absent from the workspace until they contain real implementation code and a capability manifest admits them. This avoids empty workspace placeholders that look like dead or incomplete production crates.

Closure marker: vac-provider-core is the only active provider implementation crate.
