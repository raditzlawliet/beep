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
      is_inline: q.is_inline,
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
      is_inline: f.is_inline,
    })),
    form_multipart: (pr.form_multipart ?? []).map((f) => ({
      key: f.key,
      value: f.value,
      enabled: f.enabled !== false,
      field_type: f.field_type ?? "text",
      content_type: f.content_type ?? "",
      is_inline: f.is_inline,
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
    url: (() => {
      // Strip query string. RequestParamsTab may have merged params into the URL.
      // Rust's parser does its own split_url_query.
      const q = form.url.indexOf("?");
      return q >= 0 ? form.url.slice(0, q) : form.url;
    })(),
    headers: form.headers
      .filter((h) => !h.auto)
      .map((h) => ({ key: h.key, value: h.value, enabled: h.enabled })),
    query_params: (form.query_params ?? [])
      .filter((q) => q.key)
      .map((q) => {
        const baseQ = base.query_params?.find((bq) => bq.key === q.key);
        return {
          key: q.key,
          value: q.value,
          enabled: q.enabled !== false,
          // Preserve base is_inline. Disabled params force multiline.
          // Once is_inline is false, it never goes back to true.
          is_inline: baseQ
            ? q.enabled === false
              ? false
              : baseQ.is_inline
            : true,
        };
      }),
    body: form.raw_body ?? form.body,
    body_mode: form.body_mode ?? "none",
    http_version:
      form.http_version === "Auto"
        ? null
        : `HTTP/${form.http_version === "Http1" ? "1.1" : "2"}`,
    form_urlencoded: (form.form_urlencoded ?? [])
      .filter((f) => f.key)
      .map((f) => {
        const baseF = base.form_urlencoded?.find((bf) => bf.key === f.key);
        return {
          key: f.key,
          value: f.value,
          enabled: f.enabled !== false,
          field_type: f.field_type ?? "text",
          content_type: f.content_type ?? "",
          is_inline: baseF
            ? f.enabled === false
              ? false
              : baseF.is_inline
            : true,
        };
      }),
    form_multipart: (form.form_multipart ?? [])
      .filter((f) => f.key)
      .map((f) => {
        const baseF = base.form_multipart?.find((bf) => bf.key === f.key);
        return {
          key: f.key,
          value: f.value,
          enabled: f.enabled !== false,
          field_type: f.field_type ?? "text",
          content_type: f.content_type ?? "",
          is_inline: baseF
            ? f.enabled === false
              ? false
              : baseF.is_inline
            : true,
        };
      }),
  };
}

// httpRequestToContent serialize an HttpRequest into .http file text.
// Used for standalone requests (non-file tabs).
export function httpRequestToContent(req: HttpRequest): string {
  const lines: string[] = [];

  // Title
  lines.push(`### ${req.method} ${req.url}`);

  // Request line with inline enabled params
  const enabledParams = (req.query_params ?? []).filter(
    (q) => q.enabled && q.key,
  );
  const urlWithQuery =
    enabledParams.length > 0
      ? `${req.url}?${enabledParams.map((q) => `${encodeURIComponent(q.key)}=${encodeURIComponent(q.value)}`).join("&")}`
      : req.url;
  lines.push(`${req.method} ${urlWithQuery}`);

  // Disabled params as multiline
  for (const q of (req.query_params ?? []).filter((q) => !q.enabled && q.key)) {
    lines.push(`    //- &${q.key}=${q.value}`);
  }

  // Headers (enabled and disabled)
  for (const h of req.headers) {
    if (h.auto) continue;
    lines.push(h.enabled ? `${h.key}: ${h.value}` : `//- ${h.key}: ${h.value}`);
  }

  // Body
  if (req.raw_body || req.body) {
    lines.push("");
    lines.push(req.raw_body || req.body || "");
  }

  return lines.join("\n") + "\n";
}
