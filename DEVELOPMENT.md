# Development

## Tech Stack & Prerequisites

### Required

| Tool           | Purpose                         |
| -------------- | ------------------------------- |
| **Rust**       | All backend crates              |
| **Node.js 22** | GUI frontend (SvelteKit + Vite) |

### Rust Toolchain

```bash
rustup update stable

# Cross-compilation targets (for build.sh)
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
```

### GUI System Dependencies (Tauri 2)

Same as Tauri 2 prerequisites. See the official guide:
https://v2.tauri.app/start/prerequisites/

## Running

### GUI

First, install frontend dependencies:

```bash
# Via make
make setup-gui

# or
cd crates/beep-gui
npm install
```

Run with hot-reload:

```bash
# Via make
make run-gui

# or
cd crates/beep-gui
npm run tauri dev
```

#### AI agent access (MCP bridge)

`npm run tauri:mcp` (opt-in) builds with `tauri-plugin-mcp-bridge` (WebSocket on 127.0.0.1:9223). It lets an MCP client (Claude Code via `.mcp.json`, server `@hypothesi/tauri-mcp-server`) take screenshots, read the DOM, click, and watch IPC.

```bash
cd crates/beep-gui
npm run tauri:mcp   # --features mcp-bridge + src-tauri/tauri.dev.conf.json (withGlobalTauri, mcp-bridge capability)
```

The plugin is an optional cargo feature (`mcp-bridge`), so plain `npm run tauri dev` / `npm run tauri:dev`, `cargo build` and release builds do not compile it, register it, grant its capability, or set `withGlobalTauri`. Registration also needs a debug build (`cfg(all(debug_assertions, feature = "mcp-bridge"))`). `.mcp.json` uses `cmd /c npx` (Windows); on macOS/Linux use `npx` as the command.

### CLI

You can run `beep` via cargo.

```bash
# Via make
make run-cli ARGS="https://httpbingo.org/get"

# Via cargo
cargo run -p beep -- https://httpbingo.org/get
cargo run -p beep -- POST https://httpbingo.org/post -b '{"title": "Post"}'
cargo run -p beep -- -H "Content-Type: application/json" PUT https://httpbingo.org/put -b '{"key":"value"}'

# Via built binary
./beep https://httpbingo.org/get
./beep POST https://httpbingo.org/post -b '{"title": "Post"}'
```

### TUI (Coming Soon)

TODO; The interactive terminal UI (`beep-tui` crate) is under active development.

## Building

We can build with ease using make `make build-gui TARGET=all/gui/tui/cli PLATFORM=windows/linux/macos`

All builds produce platform-specific bundles `builds` folder.

### GUI

Example build gui windows

```bash
make build TARGET=gui PLATFORM=windows
```

### CLI

```bash
make build TARGET=cli PLATFORM=windows

# or using cargo
cargo build --release -p beep
```

### TUI (Coming Soon)

TODO

### Multi-Platform Build Script

We already have a multi-platform build script that builds beep (CLI) and beep-gui (GUI) for Windows, Linux, and macOS (x64):

```bash
chmod +x build.sh
./build.sh
```

## Project Organization

### beep-core

All beep core functionality, including HTTP client, request history, and data models are in `crates/beep-core/`.

### beep-gui (Desktop GUI)

All beep GUI functionality is in `crates/beep-gui/`.

- **Rust Backend**: source available in `crates/beep-gui/src-tauri/`.
- **Frontend (SvelteKit + Vite + daisyUI)**: source available in `crates/beep-gui/src-tauri/`.

### beep (CLI)

All beep CLI functionality is in `crates/beep/`.

### beep-tui (Coming Soon)

All beep TUI functionality is in `crates/beep-tui/`.

## Common Issues

TODO

## Spec Implementation

Beep currently implements the features defined in the [HTTP/REST file format](./SPEC.md). The checklist below summarizes the parts that have already been implemented across the parser/serializer, sender/executor, and GUI.

| #     | Feature                                                   | Parser | Executor | GUI     |
| ----- | --------------------------------------------------------- | ------ | -------- | ------- |
| 2     | Comments `//` / `//-` disabled items preserved            | ✅     | -        | -       |
| 3     | Request Separator `###` + optional title                  | ✅     | -        | -       |
| 4     | Request Line `METHOD URL [HTTP/Version]`                  | ✅     | ✅       | ✅      |
| 5     | Headers `Key: Value`                                      | ✅     | ✅       | ✅      |
| 5.1   | Disabling Headers `//-`, excluded from sent request       | ✅     | ✅       | ✅      |
| 5.1.1 | Disabling Auto Headers `//- @headerAuto Key`              | ✅     | ✅       | ✅      |
| 5.2   | Authorization (Basic auto-encode, Bearer)                 | ✅     | ✅       | ✅      |
| 6.1   | `@body` Directive                                         | ✅     | ✅       | ✅      |
| 6.2   | JSON body                                                 | ✅     | ✅       | ✅      |
| 6.3   | XML body                                                  | ✅     | ✅       | ✅      |
| 6.4   | Form URL Encoded (single/multiline, disabled fields)      | ✅     | ✅       | ✅      |
| 6.5   | Multipart Form Data (with disabled fields)                | ✅     | ✅       | ✅      |
| 6.6   | Plain Text / Raw body                                     | ✅     | ✅       | ✅      |
| 6.7   | No Body                                                   | ✅     | ✅       | ✅      |
| 6.8   | Body from External File `< ./path`                        | TODO   | TODO     | TODO    |
| 7.1   | Query String Inline                                       | ✅     | ✅       | ✅      |
| 7.2   | Query String Multiline                                    | ✅     | ✅       | ✅      |
| 7.3   | Disabling Query Params `//-`                              | ✅     | ✅       | ✅      |
| 8.1   | `req.vars` Request Scope (script)                         | -      | ✅       | ✅      |
| 8.2   | `client.vars` Session Scope (persist across requests)     | -      | ✅       | ✅ [1]  |
| 8.3   | `@var` in Request Block (static, request scope)           | ✅     | ✅       | ✅ [3]  |
| 8.4   | `@var` File-Level Scope                                   | ✅     | ✅       | ✅      |
| 8.5   | `.beep/vars.json` Folder cascade                          | -      | ✅       | ✅      |
| 8.6   | Variable Interpolation `{{var}}` (trim, nested, cycles)   | -      | ✅       | ✅      |
| 8.7   | Dynamic Variables `{{$guid}}` etc.                        | -      | ✅       | ✅ [4]  |
| 9.1   | Pre-Request Script (`< {%` inline / `< ./file.js`)        | ✅     | ⚠️Basic  | ⚠️Basic |
| 9.2   | Post-Request Script (`> {%` inline / `> ./file.js`)       | ✅     | ⚠️Basic  | ⚠️Basic |
| 9.3   | Script Globals (`client`, `req`, `res`, `beep`)           | -      | ⚠️ [2]   | -       |
| 10    | Response Redirect `>>` / `>>!`                            | TODO   | TODO     | TODO    |

Notes:

- [1] Kept app-wide in `AppState.client_vars` (in-memory). Read-only list + Clear in the File Variables panel.
- [2] `client.test` / `client.assert` not implemented. No `crypto` global (e.g. `crypto.randomUUID()`).
- [3] Edit in code mode or the request form "Vars" tab; form edits rewrite only the `@` lines.
- [4] `$processEnv` asks Allow / Cancel before send (session allow-list).
- Scripts (9.1 / 9.2) "Basic": console output is not returned to the GUI; script errors are only visible when reopening the entry from history. CLI does not run scripts (no `.http` file input).
