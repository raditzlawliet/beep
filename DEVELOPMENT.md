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

Currently, beep implements the [HTTP/REST file format](./SPEC.md). Below is the checkist spec already implement in the parser/serializer, sender/executor and GUI related.

| #   | Feature                                              | Parser | Executor | GUI  |
| --- | ---------------------------------------------------- | ------ | -------- | ---- |
| 2   | Comments `//` / `//-` disabled items preserved       | ✅     | -        | -    |
| 3   | Request Separator `###` + optional title             | ✅     | -        | -    |
| 4   | Request Line `METHOD URL [HTTP/Version]`             | ✅     | ✅       | ✅   |
| 5   | Headers `Key: Value`                                 | ✅     | ✅       | ✅   |
| 5.1 | Disabling Headers `//-`, excluded from sent request  | ✅     | ✅       | ✅   |
| 6.1 | JSON body                                            | ✅     | ✅       | ✅   |
| 6.2 | XML body                                             | ✅     | ✅       | ✅   |
| 6.3 | Form URL Encoded (single/multiline, disabled fields) | ✅     | ✅       | ✅   |
| 6.4 | Multipart Form Data (with disabled fields)           | ✅     | ✅       | ✅   |
| 6.5 | Plain Text / Raw body                                | ✅     | ✅       | ✅   |
| 6.6 | No Body                                              | ✅     | ✅       | ✅   |
| 6.7 | Body from External File `< ./path`                   | TODO   | TODO     | TODO |
| 7.1 | Query String Inline                                  | ✅     | ✅       | ✅   |
| 7.2 | Query String Multiline                               | ✅     | ✅       | ✅   |
| 7.3 | Disabling Query Params `//-`                         | ✅     | ✅       | ✅   |
| 8.4 | `@var` File-Level Scope                              | ✅     | TODO     | ✅   |
| 8.5 | `.beep/vars.json` Folder cascade                     | TODO   | TODO     | TODO |
| 8.7 | Variable Interpolation `{{var}}`                     | -      | TODO     | TODO |
| 8.8 | Dynamic Variables `{{$guid}}` etc.                   | -      | TODO     | TODO |
| 9.1 | Pre-Request Script (`< {%` inline / `< ./file.js`)   | ✅     | TODO     | TODO |
| 9.2 | Post-Request Script (`> {%` inline / `> ./file.js`)  | ✅     | TODO     | TODO |
| 9.3 | Script Globals (`client`, `request`, `response`)     | -      | TODO     | TODO |
| 10  | Response Redirect `>>` / `>>!`                       | TODO   | TODO     | TODO |
