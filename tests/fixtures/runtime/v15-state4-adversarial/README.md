# VAC Runtime v1.5 State4 adversarial fixtures

These fixtures encode the State4 manual blockers as source/static SV cases. They are not Cargo tests; they assert that the real agent/MCP source path no longer trusts tool-supplied policy, no longer trusts LLM-supplied patch preimages, gates `view` through read-plan or approval, and keeps evidence refs as Git object IDs when using Git refs.
