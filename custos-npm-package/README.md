# Custos CLI (NPM Wrapper)

Custos AI Agent with Dual-Cognitive Architecture (System 1 & System 2).

## Installation

```bash
npm install -g custos-cli
```

Or run directly without installation:

```bash
npx custos-cli
```

## Features

- **High-Performance Native Core**: Packaged from compiled Rust binaries for lightning-fast execution.
- **Smart Update Notifications**: Automatically notifies you when a newer version of Custos is released.
- **Non-blocking & Zero-Latency**: Update checks run asynchronously in the background and are cached locally for 24 hours so your CLI commands are never delayed.
- **Clean Output Protection**: Update banners are automatically suppressed in `--json` mode, CI pipelines, and piped outputs to keep automation clean.

## Update Notification

When a new version is available on npm registry, a notification box is displayed upon command completion:

```text
╭───────────────────────────────────────────────────────────────────╮
│                                                                   │
│   Update available 0.1.0-alpha → 0.2.0                            │
│   Run npm i -g custos-cli to update to the latest version         │
│                                                                   │
│   Changelog: https://github.com/civi0411/Custos/releases         │
│                                                                   │
╰───────────────────────────────────────────────────────────────────╯
```

## Environment Variables

| Variable | Description |
|---|---|
| `CUSTOS_NO_UPDATE_NOTIFIER` | Set to `1` or `true` to disable update notifications completely. |
| `CUSTOS_SIMULATE_UPDATE` | Set to a version string (e.g. `0.2.0`) to simulate an update banner for testing. |
| `CUSTOS_UPDATE_CHECK_URL` | Override the default NPM registry URL for checking releases. |
