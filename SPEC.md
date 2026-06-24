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

---

## 6. Request Body

A **blank line** separates headers from the body. Body type is determined by `Content-Type`.

### 6.1 JSON

```http
### JSON body
POST https://api.example.com/users HTTP/1.1
Content-Type: application/json

{
    "name": "John Doe",
    "email": "john@example.com"
}
```

### 6.2 XML

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

### 6.3 Form URL Encoded

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

### 6.4 Multipart Form Data

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

### 6.5 Plain Text / Raw

```http
### Plain text body
POST https://api.example.com/logs HTTP/1.1
Content-Type: text/plain

This is raw text content.
No structure required.
```

### 6.6 No Body

Requests with no body (e.g., `GET`, `DELETE`, `HEAD`) simply omit the blank line and body.

```http
### No body
DELETE https://api.example.com/users/1 HTTP/1.1
Authorization: Bearer {{token}}

### HEAD
HEAD https://api.example.com/users HTTP/1.1

### OPTIONS
OPTIONS https://api.example.com/users HTTP/1.1
```

### 6.7 Body from External File

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

### 8.1 Overview

Beep resolves variables at request time by walking a precedence chain; highest scope wins.

```md
1. request.vars - highest, current request only
2. client.vars - session, entire app lifetime (in-memory)
3. @var in .http file - file-level static
4. .beep/vars.json - folder/project cascade, deepest wins
```

---

### 8.2 `request.vars` - Request Scope

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

### 8.3 `client.vars` - Session Scope

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

### 8.4 `@var` - File-Level Scope

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

### 8.5 `.beep/vars.json` - Folder / Project Scope

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

### 8.7 Variable Interpolation

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

### 8.8 Dynamic Variables

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

### 9.1 Script Globals

Three globals are available in every script context:

| Global     | Available  | Description                                     |
| ---------- | ---------- | ----------------------------------------------- |
| `client`   | pre + post | Session-level API                               |
| `request`  | pre + post | Current request data and request-scoped vars    |
| `response` | post only  | Response data. Throws if accessed in pre-script |

#### `client`

```javascript
client.vars.set("key", value); // set session variable (cross-request, in-memory)
client.vars.get("key"); // get session variable
client.vars.reset("key"); // reset to default value

client.test("name", () => {}); // define named test
client.assert(condition, "msg"); // assert inside a test
```

#### `request`

```javascript
request.vars.set("key", value); // set request-scoped variable (current request only)
request.vars.get("key"); // get request-scoped variable

request.url; // current request URL (string)
request.method; // HTTP method: "GET", "POST", etc.
request.headers; // request headers (object)
request.body; // request body (string or object)
```

#### `response` (post-script only)

```javascript
response.status; // HTTP status code (number): 200, 404, etc.
response.body; // parsed body (object if JSON, string otherwise)
response.headers; // response headers (object)
response.time; // response time in milliseconds (number)
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
