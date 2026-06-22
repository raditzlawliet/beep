export type HttpVersion = "Auto" | "Http1" | "Http2";

export type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "DELETE"
  | "PATCH"
  | "HEAD"
  | "OPTIONS";

export type Auth =
  | { type: "None" }
  | { type: "Basic"; username: string; password: string }
  | { type: "Bearer"; token: string }
  | { type: "ApiKey"; key: string; value: string; add_to: string };

export interface FormField {
  key: string;
  value: string;
  enabled: boolean;
  field_type: string;
  content_type: string;
}

export interface HeaderField {
  key: string;
  value: string;
  enabled: boolean;
  auto: boolean;
}

export interface QueryField {
  key: string;
  value: string;
  enabled: boolean;
}

export interface HttpRequest {
  url: string;
  method: HttpMethod;
  headers: HeaderField[];
  query_params: QueryField[];
  body: string | null;
  auth: Auth;
  body_mode?: string;
  raw_body?: string | null;
  form_urlencoded?: FormField[];
  form_multipart?: FormField[];
  http_version?: HttpVersion;
}

export interface ResponseSize {
  response_body: number;
  response_headers: number;
}

export interface AppConstants {
  version: string;
  platform: string;
  default_headers: [string, string][];
}

export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  elapsed_ms: number;
  size: ResponseSize;
  body_encoding?: "utf8" | "base64";
}

export interface HistoryEntrySummary {
  id: number;
  method: string;
  url: string;
  status: number | null;
  size: ResponseSize | null;
  error: string | null;
  timestamp: string;
  label: string | null;
}

export interface HistoryEntry {
  id: number;
  request: HttpRequest;
  response: HttpResponse | null;
  error: string | null;
  timestamp: string;
  label: string | null;
}

export interface ProjectNode {
  name: string;
  path: string;
  is_dir: boolean;
  children?: ProjectNode[];
}

export function defaultRequest(): HttpRequest {
  return {
    url: "",
    method: "GET",
    headers: [],
    query_params: [],
    body: null,
    auth: { type: "None" },
    body_mode: "none",
    raw_body: null,
    form_urlencoded: [],
    form_multipart: [],
    http_version: "Auto",
  };
}

// Tailwind badge background classes (for use with DaisyUI `badge`)
export function methodBadgeColor(method: HttpMethod): string {
  const map: Record<HttpMethod, string> = {
    GET: "badge-success",
    POST: "badge-warning",
    PUT: "badge-info",
    DELETE: "badge-error",
    PATCH: "badge-accent",
    HEAD: "badge-ghost",
    OPTIONS: "badge-ghost",
  };
  return map[method] || "badge-ghost";
}

// Tailwind text-color classes for inline colored method text (Postman-style)
export function methodTextColor(method: HttpMethod): string {
  const map: Record<HttpMethod, string> = {
    GET: "text-success",
    POST: "text-warning",
    PUT: "text-info",
    DELETE: "text-error",
    PATCH: "text-accent",
    HEAD: "text-base-content/70",
    OPTIONS: "text-base-content/50",
  };
  return map[method] || "text-base-content/50";
}

// text-color classes for status codes
export function statusTextColor(status: number): string {
  if (status < 200) return "text-base-content/50";
  if (status < 300) return "text-success";
  if (status < 400) return "text-info";
  if (status < 500) return "text-warning";
  return "text-error";
}

// text version of status code
export function statusText(status: number): string {
  const map: Record<number, string> = {
    200: "OK",
    201: "Created",
    204: "No Content",
    301: "Moved",
    302: "Found",
    304: "Not Modified",
    400: "Bad Request",
    401: "Unauthorized",
    403: "Forbidden",
    404: "Not Found",
    405: "Method Not Allowed",
    408: "Timeout",
    429: "Too Many Requests",
    500: "Internal Server Error",
    502: "Bad Gateway",
    503: "Unavailable",
  };
  return map[status] || "";
}
