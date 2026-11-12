# Protocol schema fixtures

These reference JSON Schemas are inputs to `tests/contract/tests/schema_compat.rs`.
The current test checks file presence and JSON object structure. Runtime validation
and compatibility guarantees are defined by the compiled contracts and their tests.

The schema IDs keep their existing URLs so references remain stable after the move.
Wire contracts used by the running application live in core and the Rust SDK.
