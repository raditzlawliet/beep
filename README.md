<h1 align="center">Beep</h1>
<p align="center">
    <em>Intuitive API Client</em>
</p>

<p align="center">
    Beep is a lightweight, cross-platform HTTP client for making API requests. Use it from the terminal or as a desktop app.
</p>

<p align="center">
  <img src="./docs/gui-preview.png" alt="Beep GUI Preview">
</p>

## Features

### Project Panel

Beep supports project folder panels to organize and manage your HTTP requests.

- Support Beep HTTP request files with `.http` or `.rest` extensions (See [Spec](./SPEC.md))
  - See [Development](./DEVELOPMENT.md) Section Spec Implementation for details
- Code <-> Form mode - Switch between code and form editor with surgical sync
- Multi-request .http files with request selector dropdown
- File-level variables (@key = value)
- File overview panel (requests list + variables editor)

### HTTP Request

Compose and send HTTP requests.

- **HTTP Methods** - All standard HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- **HTTP Version** - HTTP/1.1 and HTTP/2
- **Query Parameters** - Fully control Query Parameters
- **Authentication** - No Auth, Basic Auth, Bearer tokens
- **Headers** - Fully control Headers
- **Request Body Editor** - Edit request body with syntax highlighting
- **Form Data Editor** - Edit form data tables
- **Disable/Enable** - Toggle headers, query params, and form fields individually and preserve state
- **Request History** - Click-to-load history entries into form.
- **Response Viewer** - View response headers and body

### Coming soon

Features:

- Request: Multipart Form Data editor
- Request: Cookies
- Request: Follow Redirects
- Request: Test Scripts
- Project: Environment
- Project|Request: Variables & Prompt Variables
- Project: Save Response
- Project: Persistent History (saved per project)
- Project|Request: Individual History or Timeline by Request
- Git Integration
- Built-in AI Chat
- Built-in MCP Server
- Debugging Tools
- Secrets Management

File Supports:

- `.http` - Http Standard + Beep Extension
  - Syntax Highlighter
- `.yml` - Run/Edit Open Collection Spec
- Run/Convert cURL to Request (directly on `.http`)
- Run directly from Code (using |> play button)
- ...

Request Supports:

- GraphQL Request
- WebSocket Request
- gRPC Request
- ...

## Usage

### Desktop GUI

Run the desktop GUI based on your platform and start making requests.

#### Requirements

Desktop builds on top of Tauri, so same as Tauri requirements.

- **WebView2**

### CLI

Coming soon

### TUI

Coming soon

## Download

You can try latest pre-built binaries from the [releases page](https://github.com/raditzlawliet/beep/releases).

> v1 Stable version? When When...

## Specification

Beep use file based `.http` standard HTTP request syntax with Beep extensions. You can read the [specification](SPEC.md) for more details.

> Possible in future to read `.yaml`/`.yml`/`.json` OpenAPI specs, `.bru` Bruno specs and others related API Client specs.

## Contributors

Contributions are welcome! Check the [Issues](https://github.com/raditzlawliet/beep/issues) page or open a Pull Request.

For build instructions, architecture details, and local development, see [DEVELOPMENT.md](DEVELOPMENT.md).

<div align="center">
    <a href="https://github.com/raditzlawliet/beep/graphs/contributors">
        <img src="https://contrib.rocks/image?repo=raditzlawliet/beep" />
    </a>
</div>

## License

Apache-2.0
