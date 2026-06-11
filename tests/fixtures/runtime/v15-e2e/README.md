# VAC runtime v1.5 E2E fixtures

These fixtures validate the VAC bounded runtime contract without requiring a Rust toolchain. They model the required control-flow for an agent session:

1. compiled JSON registry intake,
2. Pre-Plan gate,
3. artifact lock for task/spec/todo,
4. Pre-Patch gate,
5. Pre-Command gate,
6. Post-Validation gate,
7. v1.5 completion lock.

They are static SV fixtures, not a substitute for the later Cargo unit/integration test loop.
