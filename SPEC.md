# Beep HTTP File Format Specification

Version: 0.1.0

## Overview

Beep uses the `.http` (or `.rest`) file format as its collection format. The format is based on the [JetBrains HTTP Request in Editor Specification](https://github.com/JetBrains/http-request-in-editor-spec) as the **base standard**, with selective extensions borrowed from [vscode-restclient](https://github.com/Huachao/vscode-restclient) and Beep-specific additions.

A single `.http` file may contain **multiple requests**, separated by `###`. One file typically represents one domain or resource (e.g., `auth.http`, `users.http`).

---

## 1. File Structure

```http
[file-level variables]

### [optional request title]
[// optional comments]
[pre-request script]
METHOD URL [HTTP/Version]
[headers]
[blank line]
[body]
[post-request script]
[response redirect]

###
...
```

Each `###` marks the start of a new request block. The `###` may optionally be followed by a request title on the same line.

---

## 2. Comments

Use `//` for comments anywhere in the file. Comments by default are ignored by the parser except some case.

Lines starting with `//- ` (double slash + dash) are **disabled items**, not comments; they are parsed and preserved across save/load. See the sections below for disabled headers, query params, and form fields.

```http
// This is a file-level comment

### Create user
// This comment is inside a request block
POST https://api.example.com/users HTTP/1.1
// Comments between headers are allowed
Content-Type: application/json

{
    "name": "John"  // inline body comments are NOT parsed (JSON doesn't support them)
}
```

---

## 3. Request Separator

Use `###` to delimit requests. An optional title may follow on the same line.

```http
### Get all users
GET https://api.example.com/users HTTP/1.1

###
GET https://api.example.com/users/1 HTTP/1.1

### Delete user
DELETE https://api.example.com/users/1 HTTP/1.1
```

---

## 4. Request Line

```
METHOD URL [HTTP/Version]
```

- `METHOD` - any valid HTTP method: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`
- `URL` - full URL, may include `{{variables}}`
- `HTTP/Version` - optional, defaults to `HTTP/1.1` if omitted

```http
### With version
GET https://api.example.com/users HTTP/1.1

### Without version
GET https://api.example.com/users
```

---

## 5. Headers

Headers follow the request line, one per line, in standard `Key: Value` format.

```http
### Authenticated request
GET https://api.example.com/me HTTP/1.1
Accept: application/json
Authorization: Bearer {{token}}
X-Request-ID: {{$guid}}
```

### 5.1 Disabling Headers

Prefix a header with `//- ` to disable it. Disabled headers are **preserved in the file** and survive save/load roundtrips, but are **excluded** from the sent request.

```http
### Request with disabled headers
GET https://api.example.com/users HTTP/1.1
Accept: application/json
Authorization: Bearer {{token}}
//- X-Debug: true
//- X-Forwarded-For: 10.0.0.1
//- Cache-Control: no-cache
```

In the example above, only `Accept` and `Authorization` are sent. `X-Debug`, `X-Forwarded-For`, and `Cache-Control` are disabled and excluded from the request.

### 5.1.1 Disabling Auto-Generated Headers

Beep sends several auto-generated headers on every request by settings (`Accept`, `Accept-Encoding`, `User-Agent`, `Connection`). To disable a specific auto-generated header via code, use the `//- @headerAuto <Key>` directive:

```http
### Request with disabled auto headers
GET https://api.example.com/data HTTP/1.1
//- @headerAuto Connection
//- @headerAuto Accept-Encoding
Accept: text/html
```

In the example above:

- `Connection` is excluded from the request.
- `Accept-Encoding` is excluded from the request.
- `Accept: text/html` overrides the default `Accept: */*`.
- The remaining auto-generated headers (`User-Agent`) are sent as usual.

When re-enabled in the UI, the `//- @headerAuto Key` line is removed from the file.

### 5.2 Authorization

The `Authorization` header is the single source of truth for auth. Beep detects common schemes and provides a dedicated Auth tab in the GUI for editing. Credentials are stored as plain text in the source file; base64 encoding for Basic auth happens transparently at execution time.

#### 5.2.1 Basic Auth

Basic authentication credentials are stored as plain text (`user:passwd`). Beep auto-encodes them to base64 at execution time. Three formats are accepted:

```http
### Plain-text user:passwd - auto base64-encoded at execution
GET https://httpbin.org/basic-auth/user/passwd HTTP/1.1
Authorization: Basic user:passwd

### Pre-encoded base64 - passed through as-is
Authorization: Basic dXNlcjpwYXNzd2Q=

### Space-separated credentials
Authorization: Basic user passwd
```

#### 5.2.2 Bearer Token

Bearer tokens are passed through directly with no additional processing.

```http
GET https://api.example.com/me HTTP/1.1
Authorization: Bearer {{token}}
```

#### 5.2.3 Other Schemes

Unknown schemes are sent verbatim as an `Authorization` header.

```http
GET https://api.example.com/data HTTP/1.1
Authorization: CustomScheme credentials-here
```

---

## 6. Request Body

A **blank line** separates headers from the body. `Content-Type` is an independent HTTP header: it is sent exactly as written and may accompany an empty body or a body with a different representation.

Beep selects the body editor and serializer in this order:

1. The request-level `// @body <kind>` directive.
2. The first enabled `Content-Type` header.
3. Fallback: `none` when body is empty, `raw/text` when body has content.

### 6.1 `@body` Directive

Use `// @body <kind>` in the header section to explicitly select how Beep parses, edits, and serializes the body without changing `Content-Type`.

Supported kinds are `none`, `raw/json`, `raw/xml`, `raw/html`, `raw/text`, `form-urlencoded`, and `form-multipart`. Raw kinds send the body text unchanged. Form kinds use Beep's structured form parser and serializer. Use `none` to explicitly mark a request as having no body.

```http
### Keep a custom media type while editing JSON
POST https://api.example.com/events
Content-Type: application/vnd.example.event+json
// @body raw/json

{"event":"created"}
```

An empty body is represented by no content after the blank line. It is independent from both `Content-Type` and `// @body`. Use `// @body none` to explicitly declare no body should be sent, which suppresses the body editor and clears any body content.

### 6.2 JSON

```http
### JSON body
POST https://api.example.com/users HTTP/1.1
Content-Type: application/json

{
    "name": "John Doe",
    "email": "john@example.com"
}
```

### 6.3 XML

```http
### XML body
POST https://api.example.com/users HTTP/1.1
Content-Type: application/xml

<?xml version="1.0" encoding="UTF-8"?>
<user>
    <name>John Doe</name>
    <email>john@example.com</email>
</user>
```

### 6.4 Form URL Encoded

```http
### Single line
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

username=john&password=secret&remember=true

### Multiline (more readable)
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

username=john
&password=secret
&remember=true
```

**Disabling form fields:** Prefix a field with `//- ` to disable it.

```http
### Form with disabled fields
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

username=john
&password=secret
//- &remember=true
//- &redirect_uri=/dashboard
```

In the example above, `username` and `password` are sent. `remember` and `redirect_uri` are disabled and excluded from the request body. Only multiline support disabled option.

### 6.5 Multipart Form Data

```http
### Multipart
POST https://api.example.com/users/1/avatar HTTP/1.1
Content-Type: multipart/form-data; boundary=boundary

--boundary
Content-Disposition: form-data; name="name"

John Doe
--boundary
Content-Disposition: form-data; name="avatar"; filename="photo.png"
Content-Type: image/png

< ./assets/photo.png
--boundary--
```

**Boundary:** The boundary value is declared in the `Content-Type` header as `boundary=<value>` and used in body separators as `--<value>`. When no `; boundary=...` parameter is present, Beep auto-generates a boundary at execution time.

**Content-Type per field:** Each multipart part may optionally include a `Content-Type` header. Three states:

- `Content-Type: image/png` explicit MIME type
- `Content-Type: ` (empty value after colon) auto, Beep decides (`application/octet-stream` for files, `text/plain` for text)
- _(no `Content-Type` line)_ not set, no `Content-Type` header sent for this part

**File fields:** File content is read from the path on the `<` line (relative or absolute). The executor reads the file at send time. An empty line separates part headers from the `<` directive.

**Disabling multipart fields:** Prefix every line of the disabled field's block with `//- `, including the boundary separator, headers, blank line, and value.

```http
### Multipart with disabled fields
POST https://api.example.com/users/1/profile HTTP/1.1
Content-Type: multipart/form-data; boundary=boundary

--boundary
Content-Disposition: form-data; name="display_name"

John Doe
//- --boundary
//- Content-Disposition: form-data; name="phone"
//-
//- +1-555-0000
--boundary
Content-Disposition: form-data; name="avatar"; filename="photo.png"
Content-Type: image/png

< ./assets/photo.png
--boundary--
```

In the example above, `display_name` and `avatar` are sent. `phone` is disabled and excluded. Note that disabled boundary lines use `//- --boundary` (the `//-` sigil followed by `--boundary`).

### 6.6 Plain Text / Raw

```http
### Plain text body
POST https://api.example.com/logs HTTP/1.1
Content-Type: text/plain

This is raw text content.
No structure required.
```

### 6.7 No Body

Requests with no body (e.g., `GET`, `DELETE`, `HEAD`) simply omit the blank line and body.

Use `// @body none` to explicitly declare no body when a Content-Type header is also present:

```http
### Explicit no body with Content-Type
POST https://api.example.com/events HTTP/1.1
Content-Type: application/json
// @body none

```

```http
### No body
DELETE https://api.example.com/users/1 HTTP/1.1
Authorization: Bearer {{token}}

### HEAD
HEAD https://api.example.com/users HTTP/1.1

### OPTIONS
OPTIONS https://api.example.com/users HTTP/1.1
```

### 6.8 Body from External File

Use `< ./path/to/file` to load body content from a file.

```http
### Body from file
POST https://api.example.com/users/import HTTP/1.1
Content-Type: application/json

< ./fixtures/users.json

### XML from file
POST https://api.example.com/users HTTP/1.1
Content-Type: application/xml

< ./fixtures/user.xml
```

---

## 7. Query String

### 7.1 Inline

```http
GET https://api.example.com/users?page=1&limit=20&sort=name HTTP/1.1
```

### 7.2 Multiline

Spread query params across lines using `?` for the first and `&` for subsequent params, indented below the URL.

```http
GET https://api.example.com/users HTTP/1.1
    ?page=1
    &limit=20
    &sort=name
    &filter=active
Accept: application/json
```

### 7.3 Disabling Query Params

Prefix a param with `//- ` to disable it. Disabled params can only be used in multiline format.

```http
### Query with disabled params
GET https://api.example.com/users HTTP/1.1
    ?page=1
    &limit=20
    //- &sort=name
    //- &filter=active
Accept: application/json
```

In the example above, `page` and `limit` are sent. `sort` and `filter` are disabled and excluded from the URL.

---

## 8. Variables

Beep resolves variables at request time by walking a precedence chain; highest scope wins.

```md
1. req.vars - highest, current request only
2. client.vars - session, entire app lifetime (in-memory)
3. @var in .http file - file-level static
4. .beep/vars.json - folder/project cascade, deepest wins
```

---

### 8.1 `req.vars` - Request Scope

- **Set via:** pre-request script only
- **Lives:** current request only, gone after request com<!--  -->pletes
- **Stored:** memory

```http
### Signed request
< {%
    const ts = new Date().toISOString();
    request.vars.set("timestamp", ts);
    request.vars.set("nonce", crypto.randomUUID());
%}
POST https://api.example.com/orders HTTP/1.1
X-Timestamp: {{timestamp}}
X-Nonce: {{nonce}}
Content-Type: application/json

{"product_id": 1}
```

After this request completes, `{{timestamp}}` and `{{nonce}}` no longer exist.

### 8.2 `client.vars` - Session Scope

- **Set via:** pre or post-request script
- **Lives:** entire app session, lost when app closes
- **Stored:** memory only (for now)

```http
### Login
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/json

{"username": "john", "password": "secret"}

> {%
    client.vars.set("token", response.body.token);
    client.vars.set("userId", response.body.user.id);
%}

### Use token - client.vars persists across requests
GET https://api.example.com/me HTTP/1.1
Authorization: Bearer {{token}}
```

### 8.3 `@var` - File-Level Scope

- **Set via:** declared at the top of a `.http` file
- **Lives:** file load time
- **Stored:** in the file itself

```http
@baseUrl = {{host}}/api/v1      // host resolved from .beep/vars.json
@contentType = application/json

### Use file-level vars
GET {{baseUrl}}/users HTTP/1.1
Content-Type: {{contentType}}
Authorization: Bearer {{token}}
```

`@var` is read-only at runtime; scripts cannot overwrite it. If `client.vars` has the same key, `client.vars` wins.

### 8.4 `.beep/vars.json` - Folder / Project Scope

- **Set via:** JSON files with naming `vars.json` or `_vars.json`
- **Lives:** always, loaded at project open
- **Stored:** in the file itself

#### Folder cascade

Beep walks from the `.http` file's directory up to the project root, collecting variable files. Deeper folder wins over shallower:

```
my-api/
├── .beep/
│   └── vars.json          - priority: lowest (project root)
└── collections/
    ├── .beep/
    │   └── vars.json      - priority: mid
    └── admin/
        ├── .beep/
        │   └── vars.json  - priority: highest (deepest folder)
        └── reports.http        - currently executing
```

If `host` is defined at both `collections/.beep/` and `my-api/.beep/`, the `collections/.beep/` value wins for any file inside `collections/`.

#### Multiple files at same level

Multiple `*_vars.json` files at the same level are loaded alphabetically. `vars.json` always loads last (canonical override file):

#### Format

Always start with object.

```json
{
  "host": "https://api.example.com",
  "version": "v1",
  "timeout": "30"
}
```

---

### 8.5 Variable Interpolation

Use `{{variable}}` anywhere in a request - URL, headers, body. Resolved at request time.

```http
### Interpolation in URL, headers, and body
POST {{baseUrl}}/users HTTP/1.1
Content-Type: application/json
Authorization: Bearer {{token}}
X-Request-ID: {{$guid}}

{
    "owner_id": "{{userId}}",
    "created_at": "{{$isoTimestamp}}"
}
```

---

### 8.6 Dynamic Variables

Built-in variables generated at request time. No setup required. Will be added more later as needed.

| Variable                 | Alias       | Description                        |
| ------------------------ | ----------- | ---------------------------------- |
| `{{$guid}}`              | `{{$uuid}}` | Random UUID v4                     |
| `{{$timestamp}}`         |             | Unix timestamp in seconds          |
| `{{$datetime iso8601}}`  |             | ISO 8601 datetime string           |
| `{{$randomInt min max}}` |             | Random integer between min and max |
| `{{$processEnv VAR}}`    |             | OS-level environment variable      |

```http
### Dynamic variables
POST https://api.example.com/items HTTP/1.1
Content-Type: application/json

{
    "id": "{{$guid}}",
    "created_at": "{{$datetime iso8601}}",
    "unix_ts": "{{$timestamp}}",
    "priority": "{{$randomInt 1 10}}"
}
```

---

## 9. Scripts

Scripts run JavaScript before or after a request. They can set runtime globals, perform assertions, and manipulate request variables.

### 9.1 Pre-Request Script

Runs before the request is sent.

**Inline:**

```http
### Signed request
< {%
    const ts = new Date().toISOString();
    const sig = `${ts}:${client.vars.get("secret")}`;
    request.vars.set("timestamp", ts);
    request.vars.set("signature", sig);
%}
POST https://api.example.com/orders HTTP/1.1
Content-Type: application/json
X-Timestamp: {{timestamp}}
X-Signature: {{signature}}

{
    "product_id": 1,
    "quantity": 2
}
```

**From file:**

```http
### Pre-script from file
< ./scripts/pre-sign.js
POST https://api.example.com/orders HTTP/1.1
Content-Type: application/json

{
    "product_id": 1,
    "quantity": 2
}
```

### 9.2 Post-Request Script (Response Handler)

Runs after the response is received.

**Inline:**

```http
### Login - capture token
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/json

{
    "username": "john",
    "password": "secret"
}

> {%
    client.vars.set("token", response.body.token);
    client.vars.set("userId", response.body.user.id);

    client.test("Status is 200", () => {
        client.assert(response.status === 200, "Expected 200");
    });
    client.test("Token exists", () => {
        client.assert(response.body.token !== undefined, "Token missing");
    });
%}
```

**From file:**

```http
### Post-script from file
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/json

{
    "username": "john",
    "password": "secret"
}

> ./scripts/post-login.js
```

### 9.3 Script Globals

Three globals are available (following Bruno conventions):

| Global   | Available  | Description                                      |
| -------- | ---------- | ------------------------------------------------ |
| `client` | pre + post | Session-level API (vars, test, assert)           |
| `req`    | pre + post | Current request CRUD (read + mutate before send) |
| `res`    | post only  | Response data. Throws if accessed in pre-script  |
| `beep`   | pre + post | Utility namespace (interpolate)                  |

#### `client`

```javascript
client.vars.set("key", value); // set session variable (cross-request, in-memory)
client.vars.get("key"); // get session variable
client.vars.reset("key"); // reset to default value

client.test("name", () => {}); // define named test (API shell)
client.assert(condition, "msg"); // assert inside a test (API shell)
```

#### `req`

```javascript
// Variables
req.vars.set("key", value); // set request-scoped variable
req.vars.get("key"); // get request-scoped variable

// Read methods
req.getUrl(); // current request URL
req.getMethod(); // HTTP method: "GET", "POST", etc.
req.getHeader("Content-Type"); // get a header value by name (case-insensitive)

// Read-only convenience (legacy)
req.url; // current request URL (string)
req.method; // HTTP method (string)
req.headers; // request headers (object)
req.body; // request body (string or null)

// Mutation methods (pre-script only — override the final sent request, not the UI form):
req.setUrl(newUrl); // override request URL
req.setMethod(newMethod); // override HTTP method (e.g. "POST")
req.setHeader(key, value); // add or override a request header
req.setBody(body); // override request body as string
req.deleteHeader(name); // remove a request header by name
req.deleteHeaders(jsonArray); // remove multiple headers (pass JSON array string)
```

#### `res` (post-script only)

```javascript
res.status; // HTTP status code (number): 200, 404, etc.
res.statusText; // human-readable status text: "OK", "Not Found", etc.
res.body; // parsed body (object if JSON, string otherwise)
res.headers; // response headers (object)
res.time; // response time in milliseconds (number)
res.size; // response size object { headers: number, body: number }
```

---

## 10. Response Redirect

Save the response body to a file.

- `>>` - append to file
- `>>!` - overwrite file

```http
### Save response (append)
GET https://api.example.com/users HTTP/1.1
Accept: application/json

>> ./output/users.json

### Save response (overwrite)
GET https://api.example.com/report HTTP/1.1
Accept: application/json

>>! ./output/report.json
```

Response redirect and post-script may be combined. Script runs first, then file is written.

```http
### Script + redirect
GET https://api.example.com/users HTTP/1.1
Accept: application/json

> {%
    client.test("Status is 200", () => {
        client.assert(response.status === 200, "Expected 200");
    });
%}
>>! ./output/users.json
```

---

## Full Example

A real-world `auth.http` demonstrating the complete format:

```http
// ==============================================
// Auth Collection
// ==============================================

@baseUrl = {{host}}/api/v1

// ==============================================
// Login - captures token for subsequent requests
// ==============================================

### Login
POST {{baseUrl}}/auth/login HTTP/1.1
Content-Type: application/json

{
    "username": "{{username}}",
    "password": "{{password}}"
}

> {%
    client.vars.set("token", response.body.token);
    client.vars.set("userId", response.body.user.id);
    client.vars.set("refreshToken", response.body.refresh_token);

    client.test("Login success", () => {
        client.assert(response.status === 200, "Expected 200");
        client.assert(response.body.token !== undefined, "Token missing");
    });
%}

###
// Refresh token
POST {{baseUrl}}/auth/refresh HTTP/1.1
Content-Type: application/json

{
    "refresh_token": "{{refreshToken}}"
}

> {%
    client.vars.set("token", response.body.token);
%}

###
// Get current user profile
GET {{baseUrl}}/auth/me HTTP/1.1
Authorization: Bearer {{token}}
Accept: application/json

>>! ./output/me.json

###
// Update password
PUT {{baseUrl}}/auth/password HTTP/1.1
Content-Type: application/json
Authorization: Bearer {{token}}

{
    "current_password": "{{password}}",
    "new_password": "newSecret123"
}

> {%
    client.test("Password updated", () => {
        client.assert(response.status === 200, "Expected 200");
    });
%}

###
// Logout
POST {{baseUrl}}/auth/logout HTTP/1.1
Authorization: Bearer {{token}}

> {%
    client.vars.clear("token");
    client.vars.clear("userId");
    client.vars.clear("refreshToken");
%}
```

---

## Planned

| Feature                  | Status |
| ------------------------ | ------ |
| Environment              | TBD    |
| cURL import              | TBD    |
| Persistence variables    | TBD    |
| Cookies                  | TBD    |
| Prompt variables         | TBD    |
| GraphQL `X-REQUEST-TYPE` | TBD    |
| WebSocket                | TBD    |
| gRPC                     | TBD    |
