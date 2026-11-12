# Custos Rust SDK

The shared client used by the desktop hosts and daemon integrations.

- `src/local_api` contains request envelopes, Local API bindings and the client.
- `src/profile.rs` resolves profiles, daemon discovery and attachment lifecycle.
- `src/wire_types` contains ACP compatibility request and notification types.

Run the client tests from the repository root.

```sh
cargo test -p custos-sdk --lib --offline
```

The ACP compatibility types originated from the Goose SDK. Upstream integration
provenance is recorded in `docs/development/upstream-source-map.md` at the repository root.
