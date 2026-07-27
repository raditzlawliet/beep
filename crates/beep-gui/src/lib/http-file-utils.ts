import type {
  HttpMethod,
  HttpVersion,
  ParsedRequest,
  HeaderField,
  ParsedQueryField,
  ParsedFormField,
} from "./types";

export function parseHttpVersion(v: string | null | undefined): HttpVersion {
  if (!v) return "Auto";
  const upper = v.toUpperCase();
  if (upper.startsWith("HTTP/1")) return "Http1";
  if (upper.startsWith("HTTP/2")) return "Http2";
  return "Auto";
}

// parsedToFormRequest prepares a ParsedRequest for form editing.
// Since ParsedRequest IS the form model now, this is mostly a copy with convenience defaults applied.
export function parsedToFormRequest(
  pr: ParsedRequest | undefined,
): ParsedRequest {
  if (!pr) {
    return {
      title: "",
      method: "GET",
      url: "",
      headers: [],
      query_params: [],
      body: null,
      body_mode: "none",
      form_urlencoded: [],
      form_multipart: [],
      pre_script: null,
      post_script: null,
      http_version: null,
      block_region: { start: 0, end: 0 },
      request_line_region: { start: 0, end: 0 },
      query_region: { start: 0, end: 0 },
      headers_region: { start: 0, end: 0 },
      body_region: { start: 0, end: 0 },
    };
  }
  return { ...pr };
}

// formRequestToParsed merges form edits back into the base ParsedRequest for serialization. Preserves regions from base.
export function formRequestToParsed(
  form: ParsedRequest,
  base: ParsedRequest,
): ParsedRequest {
  return {
    ...base,
    method: form.method,
    url: (() => {
      const q = form.url.indexOf("?");
      return q >= 0 ? form.url.slice(0, q) : form.url;
    })(),
    headers: form.headers
      .filter((h) => !h.auto || !h.enabled)
      .map((h) => ({
        key: h.key,
        value: h.auto && !h.enabled ? "" : h.value,
        enabled: h.enabled,
        auto: h.auto && !h.enabled,
      })),
    query_params: (form.query_params ?? [])
      .filter((q) => q.key)
      .map((q) => {
        const baseQ = base.query_params?.find((bq) => bq.key === q.key);
        return {
          key: q.key,
          value: q.value,
          enabled: q.enabled !== false,
          is_inline: baseQ
            ? q.enabled === false
              ? false
              : baseQ.is_inline
            : true,
        };
      }),
    body: form.body,
    body_mode: form.body_mode ?? "none",
    http_version: form.http_version,
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

// parsedRequestToContent serialize a ParsedRequest into .http file text.
export function parsedRequestToContent(req: ParsedRequest): string {
  const lines: string[] = [];

  lines.push(`### ${req.method} ${req.url}`);

  const enabledParams = (req.query_params ?? []).filter(
    (q) => q.enabled && q.key,
  );
  const urlWithQuery =
    enabledParams.length > 0
      ? `${req.url}?${enabledParams.map((q) => `${encodeURIComponent(q.key)}=${encodeURIComponent(q.value)}`).join("&")}`
      : req.url;
  lines.push(`${req.method} ${urlWithQuery}`);

  for (const q of (req.query_params ?? []).filter((q) => !q.enabled && q.key)) {
    lines.push(`    //- &${q.key}=${q.value}`);
  }

  for (const h of req.headers) {
    if (h.auto && h.enabled) continue; // enabled auto handled by executor
    if (h.auto) {
      lines.push(`//- @headerAuto ${h.key}`);
    } else if (h.enabled) {
      lines.push(`${h.key}: ${h.value}`);
    } else {
      lines.push(`//- ${h.key}: ${h.value}`);
    }
  }

  if (req.body) {
    lines.push("");
    lines.push(req.body || "");
  }

  return lines.join("\n") + "\n";
}
