# Custos CLI

The interactive Node terminal CLI (`bin/custos-cli.js`) connects to the Custos daemon over the local loopback API.
For the pure-Rust headless CLI binary, see [`apps/cli`](../../apps/cli/Cargo.toml) (`cargo run -p custos-app-cli --`).

## Run

```text
npm start              # interactive Node terminal CLI (bin/custos-cli.js)
cargo run -p custos-app-cli -- --help   # pure-Rust headless CLI
```

Run `npm start -- help` to see grouped commands. In interactive mode, press `/` to open the searchable command palette and use the arrow keys to navigate.

## Verify

```text
node bin/custos-cli.js help
node bin/custos-cli.js status
```
