import type {
  HttpRequest,
  HttpMethod,
  HttpVersion,
  ParsedRequest,
  HeaderField,
  Auth,
} from "./types";

function parseHttpVersion(v: string | null | undefined): HttpVersion {
  if (!v) return "Auto";
  const upper = v.toUpperCase();
  if (upper.startsWith("HTTP/1")) return "Http1";
  if (upper.startsWith("HTTP/2")) return "Http2";
  return "Auto";
}

// parsedToHttpRequest convert a parsed request from the Rust parser into a form-ready HttpRequest.
export function parsedToHttpRequest(
  pr: ParsedRequest | undefined,
): HttpRequest {
  if (!pr) {
    return {
      url: "",
      method: "GET",
      headers: [],
      query_params: [],
      body: null,
      auth: { type: "None" as const },
      body_mode: "none",
      raw_body: null,
      form_urlencoded: [],
      form_multipart: [],
    };
  }
  const headers: HeaderField[] = pr.headers.map((h) => ({
    key: h.key,
    value: h.value,
    enabled: h.enabled !== false,
    auto: false,
  }));
  return {
    url: pr.url,
    method: (pr.method || "GET") as HttpMethod,
    headers,
    query_params: (pr.query_params ?? []).map((q) => ({
      key: q.key,
      value: q.value,
      enabled: q.enabled !== false,
    })),
    body: pr.body,
    auth: { type: "None" as const } as Auth,
    body_mode: pr.body_mode ?? undefined,
    http_version: parseHttpVersion(pr.http_version),
    raw_body: pr.body,
    form_urlencoded: (pr.form_urlencoded ?? []).map((f) => ({
      key: f.key,
      value: f.value,
      enabled: f.enabled !== false,
      field_type: f.field_type ?? "text",
      content_type: f.content_type ?? "",
    })),
    form_multipart: (pr.form_multipart ?? []).map((f) => ({
      key: f.key,
      value: f.value,
      enabled: f.enabled !== false,
      field_type: f.field_type ?? "text",
      content_type: f.content_type ?? "",
    })),
  };
}

// httpRequestToParsed convert a form HttpRequest back into a ParsedRequest for serialization.
export function httpRequestToParsed(
  form: HttpRequest,
  base: ParsedRequest,
): ParsedRequest {
  return {
    ...base,
    method: form.method,
    url: form.url,
    headers: form.headers
      .filter((h) => h.enabled && !h.auto)
      .map((h) => ({ key: h.key, value: h.value, enabled: h.enabled })),
    query_params: (form.query_params ?? [])
      .filter((q) => q.key)
      .map((q) => ({
        key: q.key,
        value: q.value,
        enabled: q.enabled !== false,
      })),
    body: form.raw_body ?? form.body,
    body_mode: form.body_mode ?? "none",
    http_version:
      form.http_version === "Auto"
        ? null
        : `HTTP/${form.http_version === "Http1" ? "1.1" : "2"}`,
    form_urlencoded: (form.form_urlencoded ?? [])
      .filter((f) => f.key)
      .map((f) => ({
        key: f.key,
        value: f.value,
        enabled: f.enabled !== false,
        field_type: f.field_type ?? "text",
        content_type: f.content_type ?? "",
      })),
    form_multipart: (form.form_multipart ?? [])
      .filter((f) => f.key)
      .map((f) => ({
        key: f.key,
        value: f.value,
        enabled: f.enabled !== false,
        field_type: f.field_type ?? "text",
        content_type: f.content_type ?? "",
      })),
  };
}

// httpRequestToContent serialize an HttpRequest into .http file text.
export function httpRequestToContent(req: HttpRequest): string {
  const lines = [`### ${req.method} ${req.url}`];
  lines.push(`${req.method} ${req.url}`);
  for (const h of req.headers) {
    if (h.enabled && !h.auto) lines.push(`${h.key}: ${h.value}`);
  }
  if (req.raw_body || req.body) {
    lines.push("");
    lines.push(req.raw_body || req.body || "");
  }
  return lines.join("\n") + "\n";
}
