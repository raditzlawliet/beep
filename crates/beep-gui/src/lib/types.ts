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
  | { type: "Bearer"; token: string };

export interface HttpRequest {
  url: string;
  method: HttpMethod;
  headers: Record<string, string>;
  query_params: Record<string, string>;
  body: string | null;
  auth: Auth;
  body_mode?: string;
  body_type?: string;
}

export interface ResponseSize {
  response_body: number;
  response_headers: number;
}

export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  elapsed_ms: number;
  size: ResponseSize;
}

export interface HistoryEntry {
  id: number;
  request: HttpRequest;
  response: HttpResponse | null;
  timestamp: string;
  label: string | null;
}

export function defaultRequest(): HttpRequest {
  return {
    url: "",
    method: "GET",
    headers: {},
    query_params: {},
    body: null,
    auth: { type: "None" },
    body_mode: "none",
    body_type: "Text",
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
