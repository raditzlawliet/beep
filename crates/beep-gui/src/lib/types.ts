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
  is_inline?: boolean;
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
  is_inline?: boolean;
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

export interface Size {
  headers: number;
  body: number;
}

export interface AppConstants {
  version: string;
  platform: string;
  default_headers: [string, string][];
}

export interface SentRequest {
  url: string;
  method: string;
  headers: [string, string][];
  body: string | null;
  http_version: string;
  size: Size | null;
}

export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  elapsed_ms: number;
  size: Size;
  body_encoding?: "utf8" | "base64";
}

export interface RequestResult {
  request: SentRequest;
  response: HttpResponse;
}

export interface HistoryEntrySummary {
  id: number;
  method: string;
  url: string;
  status: number | null;
  size: Size | null;
  error: string | null;
  timestamp: string;
  label: string | null;
}

export interface HistoryEntry {
  id: number;
  request: HttpRequest;
  result: RequestResult | null;
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

// Tab types

export type TabType = "http-file" | "file";
export type ViewMode = "code" | "file" | "request";

export interface ParsedFileVariable {
  key: string;
  value: string;
}

export interface ParsedHeaderField {
  key: string;
  value: string;
  enabled: boolean;
}

export interface ParsedQueryField {
  key: string;
  value: string;
  enabled: boolean;
  is_inline: boolean;
}

export interface ParsedFormField {
  key: string;
  value: string;
  enabled: boolean;
  field_type: string;
  content_type: string;
  is_inline: boolean;
}

export interface ParsedRegion {
  start: number;
  end: number;
}

export interface ParsedRequest {
  title: string;
  method: string;
  url: string;
  headers: ParsedHeaderField[];
  query_params: ParsedQueryField[];
  body: string | null;
  body_mode: string | null;
  form_urlencoded: ParsedFormField[];
  form_multipart: ParsedFormField[];
  //
  block_region: ParsedRegion;
  request_line_region: ParsedRegion;
  query_region: ParsedRegion;
  headers_region: ParsedRegion;
  body_region: ParsedRegion;
  pre_script: string | null;
  post_script: string | null;
  http_version: string | null;
}

export interface ParsedHttpFileResult {
  variables: ParsedFileVariable[];
  requests: ParsedRequest[];
}

export interface Tab {
  id: string;
  type: TabType;
  label: string;
  filePath?: string;
  content: string;
  originalContent?: string;
  diskChanged?: boolean;
  persistent: boolean;
  cursorPos?: number;
  // http-file specific
  viewMode?: ViewMode;
  activeRequestIdx?: number;
  parsedRequests?: ParsedRequest[];
  fileVariables?: ParsedFileVariable[];
  // sub-tab state preservation
  requestFormTab?: string;
  fileOverviewTab?: string;
  lastResult?: RequestResult | null;
  // Timestamp of last activation (for MRU tab switcher ordering)
  lastActiveAt?: number;
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

export function emptyParsedRequest(): ParsedRequest {
  return {
    title: "Untitled",
    method: "GET",
    url: "https://example.com",
    headers: [],
    query_params: [],
    body: null,
    body_mode: "none",
    form_urlencoded: [],
    form_multipart: [],
    //
    block_region: { start: 0, end: 0 },
    request_line_region: { start: 0, end: 0 },
    query_region: { start: 0, end: 0 },
    headers_region: { start: 0, end: 0 },
    body_region: { start: 0, end: 0 },
    pre_script: null,
    post_script: null,
    http_version: null,
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

// Tailwind text-color classes for inline colored method text
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
export function statusTextColor(status: number | null): string {
  if (status === null) return "text-base-content/50";
  if (status < 200) return "text-base-content/50";
  if (status < 300) return "text-success";
  if (status < 400) return "text-info";
  if (status < 500) return "text-warning";
  return "text-error";
}

// text version of status code
export function statusText(status: number | null): string {
  if (status === null) return "";
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

export function isHttpFile(name: string): boolean {
  const ext = name.split(".").pop()?.toLowerCase();
  return ext === "http" || ext === "rest";
}
