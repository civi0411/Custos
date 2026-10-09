# Custos CLI

Custos provides two interfaces from this package:

- A terminal CLI for quick task and mode commands.
- A React/Tauri interface with a terminal-first workspace and visual task views.

## Run

```text
npm start              # terminal CLI
npm run dev            # browser demo with clearly labeled mock data
npm run tauri:dev      # desktop client connected through the Custos host
```

Run `npm start -- help` to see grouped commands. In interactive mode, press `/` to open the searchable command palette and use the arrow keys to navigate.

## Verify

```text
npm run build
node bin/custos-cli.js help
node bin/custos-cli.js status
```
