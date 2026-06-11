<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import type { HttpRequest, HttpMethod, Auth } from "$lib/types";
    import { methodTextColor } from "$lib/types";
    import { jsonrepair } from "jsonrepair";
    import RequestParamsTab from "$lib/components/tabs/RequestParamsTab.svelte";
    import RequestHeadersTab from "$lib/components/tabs/RequestHeadersTab.svelte";
    import RequestAuthTab from "$lib/components/tabs/RequestAuthTab.svelte";
    import RequestBodyTab from "$lib/components/tabs/RequestBodyTab.svelte";

    interface Props {
        request: HttpRequest;
        loading: boolean;
        onSend: (req: HttpRequest) => void;
        onUpdate: (req: HttpRequest) => void;
    }

    let { request, loading, onSend, onUpdate }: Props = $props();

    type Tab = "params" | "headers" | "auth" | "body";
    let activeTab = $state<Tab>("params");

    export type BodyMode = "none" | "raw";
    export type BodyType = "text" | "json";
    let bodyMode = $state<BodyMode>("none");
    let bodyType = $state<BodyType>("text");

    // Preserved raw content - survives switching between none/raw
    let rawBodyContent = $state("");

    // Sync rawBodyContent when request.body is loaded externally (e.g. history)
    $effect(() => {
        const body = request.body;
        const mode =
            (request.body_mode as BodyMode) || (body !== null ? "raw" : "none");
        const type = (request.body_type as BodyType) || "text";

        if (body !== null && body !== rawBodyContent) {
            rawBodyContent = body;
        } else if (body === null) {
            rawBodyContent = "";
        }
        bodyMode = mode;
        bodyType = type;
    });

    function beautifyJson(): string {
        try {
            const repaired = jsonrepair(rawBodyContent);
            const parsed = JSON.parse(repaired);
            return JSON.stringify(parsed, null, 2);
        } catch {
            return rawBodyContent;
        }
    }

    const METHODS: HttpMethod[] = [
        "GET",
        "POST",
        "PUT",
        "DELETE",
        "PATCH",
        "HEAD",
        "OPTIONS",
    ];

    // Core defaults loaded from backend (Accept, User-Agent, etc.)
    let defaultHeaders = $state<[string, string][]>([]);
    let showAutoHeaders = $state(false);

    invoke<[string, string][]>("get_default_headers").then((headers) => {
        defaultHeaders = headers;
    });

    // --- row types ---
    export type ParamRow = { key: string; value: string; enabled: boolean };
    export type HeaderRow = {
        key: string;
        value: string;
        enabled: boolean;
        auto: boolean;
    };
    type UrlParam = { key: string; value: string };

    function mkRow(key = "", value = "", enabled = true): ParamRow {
        return { key, value, enabled };
    }

    function rowsToObj(
        rows: { key: string; value: string; enabled: boolean }[],
    ): Record<string, string> {
        const obj: Record<string, string> = {};
        for (const r of rows) {
            if (r.enabled && r.key.trim()) obj[r.key.trim()] = r.value;
        }
        return obj;
    }

    // --- editable lists (mutated in-place, never replaced) ---
    let headerRows = $state<HeaderRow[]>([]);
    let paramRows = $state<ParamRow[]>([]);

    // --- reinit from request prop (history load / new request) ---
    let _prevQp: Record<string, string> = {};
    let _prevDefaultHeaders: [string, string][] = [];
    let _prevRequestHeaders: Record<string, string> = {};
    let _initialized = false;

    // derived state
    let hasParams = $derived(
        paramRows.some((r) => r.key.trim() || r.value.trim()),
    );

    let headerCount = $derived(
        headerRows.filter((r) => r.enabled && r.key.trim()).length,
    );

    let autoHeaderCount = $derived(
        headerRows.filter((r) => r.auto && r.enabled && r.key.trim()).length,
    );

    $effect(() => {
        const qp = request.query_params;
        const rh = request.headers;

        const coreHeaders = defaultHeaders;

        // Detect if default headers changed from async Tauri load
        const dhChanged =
            _prevDefaultHeaders.length !== coreHeaders.length ||
            _prevDefaultHeaders.some(
                ([k, v], i) =>
                    coreHeaders[i]?.[0] !== k || coreHeaders[i]?.[1] !== v,
            );

        // Detect if request.headers changed (e.g. history navigation)
        const prevKeysH = Object.keys(_prevRequestHeaders).sort().join(",");
        const curKeysH = Object.keys(rh).sort().join(",");
        const rhChanged =
            curKeysH !== prevKeysH ||
            Object.keys(rh).some((k) => rh[k] !== _prevRequestHeaders[k]);

        // Skip short-circuit on first run so tables always get populated
        if (_initialized) {
            const qpKeys = Object.keys(qp).sort().join(",");
            const prevKeys = Object.keys(_prevQp).sort().join(",");
            if (qpKeys === prevKeys && !dhChanged && !rhChanged) {
                let same = true;
                for (const k of Object.keys(qp)) {
                    if (qp[k] !== _prevQp[k]) {
                        same = false;
                        break;
                    }
                }
                if (same) return;
            }
        }
        _initialized = true;
        _prevQp = { ...qp };
        _prevDefaultHeaders = [...coreHeaders];
        _prevRequestHeaders = { ...rh };

        // Actually different - reinit table from query_params
        headerRows.length = 0;

        // GUI auto-generated headers (always available immediately)
        const existingHdrKeys = new Set(
            Object.keys(rh).map((k) => k.toLowerCase()),
        );

        // default headers
        for (const [key, value] of coreHeaders) {
            if (!existingHdrKeys.has(key.toLowerCase())) {
                headerRows.push({ key, value, enabled: true, auto: true });
            }
        }

        // Then user-defined headers
        for (const [key, value] of Object.entries(rh)) {
            headerRows.push({ key, value, enabled: true, auto: false });
        }
        ensureTempRowH();

        paramRows.length = 0;
        for (const [key, value] of Object.entries(qp)) {
            paramRows.push({ key, value, enabled: true });
        }
        ensureTempRow();
    });

    // --- build query string from URL ---
    function buildUrlWithParams(baseUrl: string, rows: ParamRow[]): string {
        try {
            const u = new URL(baseUrl);
            u.search = "";
            for (const r of rows) {
                if (r.enabled && r.key.trim())
                    u.searchParams.append(r.key.trim(), r.value);
            }
            return u.toString();
        } catch {
            const qs = rows
                .filter((r) => r.enabled && r.key.trim())
                .map(
                    (r) =>
                        `${encodeURIComponent(r.key.trim())}=${encodeURIComponent(r.value)}`,
                )
                .join("&");
            const idx = baseUrl.indexOf("?");
            const base = idx >= 0 ? baseUrl.slice(0, idx) : baseUrl;
            return qs ? `${base}?${qs}` : base;
        }
    }

    // --- ensure a temp row exists at the end (mutates in-place) ---
    function ensureTempRow() {
        const last = paramRows[paramRows.length - 1];
        if (
            paramRows.length === 0 ||
            (last && (last.key.trim() || last.value.trim()))
        ) {
            paramRows.push(mkRow());
        }
    }

    function ensureTempRowH() {
        const last = headerRows[headerRows.length - 1];
        if (
            headerRows.length === 0 ||
            (last && (last.key.trim() || last.value.trim()))
        ) {
            headerRows.push({ key: "", value: "", enabled: true, auto: false });
        }
    }

    function removeRow(list: "headers" | "params", idx: number) {
        if (list === "headers") {
            headerRows.splice(idx, 1);
            ensureTempRowH();
        } else {
            const oldRows = paramRows.map((r) => ({ ...r }));
            paramRows.splice(idx, 1);
            ensureTempRow();
            syncUrlFromParams(oldRows);
        }
    }

    function updateRow(
        list: "headers" | "params",
        idx: number,
        field: "key" | "value",
        val: string,
    ) {
        if (list === "headers") {
            const r = headerRows[idx];
            headerRows[idx] = { ...r, [field]: val };
            ensureTempRowH();
        } else {
            const oldRows = paramRows.map((r) => ({ ...r }));
            const r = paramRows[idx];
            paramRows[idx] = { ...r, [field]: val };
            ensureTempRow();
            syncUrlFromParams(oldRows);
        }
    }

    function toggleRow(list: "headers" | "params", idx: number) {
        if (list === "headers") {
            const r = headerRows[idx];
            headerRows[idx] = { ...r, enabled: !r.enabled };
        } else {
            const oldRows = paramRows.map((r) => ({ ...r }));
            const r = paramRows[idx];
            paramRows[idx] = { ...r, enabled: !r.enabled };
            syncUrlFromParams(oldRows);
        }
    }

    // --- ordered URL param helpers ---
    function parseUrlParamsOrdered(url: string): UrlParam[] {
        try {
            return [...new URL(url).searchParams].map(([k, v]) => ({
                key: k,
                value: v,
            }));
        } catch {
            const idx = url.indexOf("?");
            if (idx < 0) return [];
            const qs = url.slice(idx + 1);
            const out: UrlParam[] = [];
            for (const part of qs.split("&")) {
                const eq = part.indexOf("=");
                if (eq >= 0) {
                    out.push({
                        key: decodeURIComponent(part.slice(0, eq)),
                        value: decodeURIComponent(part.slice(eq + 1)),
                    });
                } else if (part.trim()) {
                    out.push({
                        key: decodeURIComponent(part),
                        value: "",
                    });
                }
            }
            return out;
        }
    }

    function buildUrlFromOrdered(baseUrl: string, params: UrlParam[]): string {
        try {
            const u = new URL(baseUrl);
            u.search = "";
            for (const p of params) u.searchParams.append(p.key, p.value);
            return u.toString();
        } catch {
            const qs = params
                .map(
                    (p) =>
                        `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`,
                )
                .join("&");
            const idx = baseUrl.indexOf("?");
            const base = idx >= 0 ? baseUrl.slice(0, idx) : baseUrl;
            return qs ? `${base}?${qs}` : base;
        }
    }

    function enabledFromRows(rows: ParamRow[]): (ParamRow & { pos: number })[] {
        const out: (ParamRow & { pos: number })[] = [];
        let pos = 0;
        for (const r of rows) {
            if (r.enabled && r.key.trim()) {
                out.push({ ...r, pos });
                pos++;
            }
        }
        return out;
    }

    // --- surgical table -> URL sync ---
    function syncUrlFromParams(oldRows: ParamRow[]) {
        const newRows = paramRows;
        const oldEP = enabledFromRows(oldRows);
        const newEP = enabledFromRows(newRows);
        const urlP = parseUrlParamsOrdered(request.url);

        let aligned = oldEP.length === urlP.length;
        if (aligned) {
            for (let i = 0; i < oldEP.length; i++) {
                if (
                    oldEP[i].key !== urlP[i].key ||
                    oldEP[i].value !== urlP[i].value
                ) {
                    aligned = false;
                    break;
                }
            }
        }

        if (aligned && oldEP.length !== newEP.length) {
            const newParams: UrlParam[] = newEP.map((r) => ({
                key: r.key,
                value: r.value,
            }));
            const newUrl = buildUrlFromOrdered(request.url, newParams);
            if (newUrl !== request.url) {
                onUpdate({ ...request, url: newUrl });
            }
            return;
        }

        if (aligned && oldEP.length === newEP.length) {
            let changed = false;
            const newParams = urlP.slice();
            for (let i = 0; i < oldEP.length; i++) {
                if (
                    oldEP[i].key !== newEP[i].key ||
                    oldEP[i].value !== newEP[i].value
                ) {
                    newParams[i] = {
                        key: newEP[i].key,
                        value: newEP[i].value,
                    };
                    changed = true;
                }
            }
            if (changed) {
                const newUrl = buildUrlFromOrdered(request.url, newParams);
                if (newUrl !== request.url) {
                    onUpdate({ ...request, url: newUrl });
                }
            }
            return;
        }

        // Fallback
        const realRows = newRows.filter((r) => r.key.trim() || r.value.trim());
        const newUrl = buildUrlWithParams(request.url, realRows);
        if (newUrl !== request.url) {
            onUpdate({ ...request, url: newUrl });
        }
    }

    function emitUpdate(overrides: Partial<HttpRequest>) {
        const base: HttpRequest = {
            ...request,
            headers: rowsToObj(headerRows.filter((r) => !r.auto)),
            body_mode: bodyMode,
            body_type: bodyType,
            ...overrides,
        };
        // Only include query_params when NOT a URL-only change
        // (URL is source of truth for params; duplicates survive in URL but not Record)
        if (!overrides.url) {
            base.query_params = rowsToObj(paramRows);
        }
        onUpdate(base);
    }

    function handleSend() {
        const req: HttpRequest = {
            ...request,
            headers: rowsToObj(headerRows),
            query_params: rowsToObj(paramRows),
            body_mode: bodyMode,
            body_type: bodyType,
        };
        onSend(req);
    }

    // --- auto-sync: URL -> params table (called from URL input oninput) ---
    function syncTableFromUrl(url: string) {
        const urlP = parseUrlParamsOrdered(url);

        // Build map of current table rows
        const tableMap = new Map<string, ParamRow>();
        for (const r of paramRows) {
            if (r.key.trim()) tableMap.set(r.key.trim(), r);
        }

        const newRows: ParamRow[] = [];
        for (const p of urlP) {
            const existing = tableMap.get(p.key);
            newRows.push({
                key: p.key,
                value: p.value,
                enabled: existing ? existing.enabled : true,
            });
        }
        // Preserve disabled rows not in URL
        for (const r of paramRows) {
            if (
                !r.enabled &&
                r.key.trim() &&
                !urlP.some((p) => p.key === r.key.trim())
            ) {
                newRows.push({ ...r });
            }
        }

        paramRows.length = 0;
        for (const r of newRows) paramRows.push(r);
        ensureTempRow();
    }
</script>

<div class="card rounded-none bg-base-100 h-full">
    <div class="card-body p-0 flex flex-col min-h-0">
        <!-- method + url + send row -->
        <div class="join w-full p-2 pb-0">
            <select
                class="join-item select select-bordered select-sm w-28 font-mono font-bold {methodTextColor(
                    request.method,
                )}"
                value={request.method}
                onchange={(e) =>
                    emitUpdate({
                        method: (e.target as HTMLSelectElement)
                            .value as HttpMethod,
                    })}
            >
                {#each METHODS as m}
                    <option value={m} class={methodTextColor(m)}>{m}</option>
                {/each}
            </select>
            <input
                type="text"
                class="join-item input input-bordered input-sm flex-1 font-mono"
                placeholder="https://api.example.com/endpoint"
                value={request.url}
                oninput={(e) => {
                    const newUrl = (e.target as HTMLInputElement).value;
                    syncTableFromUrl(newUrl);
                    emitUpdate({ url: newUrl });
                }}
                onkeydown={(e) => {
                    if (e.key === "Enter" && !loading && request.url.trim())
                        handleSend();
                }}
            />
            <button
                class="join-item btn btn-sm btn-primary"
                onclick={handleSend}
                disabled={loading || !request.url.trim()}
            >
                {#if loading}
                    <span class="loading loading-spinner loading-xs"></span>
                {/if}
                Send
            </button>
        </div>
        <div class="border-b border-b-base-content/10"></div>

        <!-- tabs -->
        <div role="tablist" class="tabs tabs-bordered tabs-xs px-1">
            {#each ["params", "auth", "headers", "body"] as tab}
                <button
                    role="tab"
                    class="tab capitalize gap-1.5 {activeTab === tab
                        ? 'tab-active'
                        : ''}"
                    onclick={() => (activeTab = tab as Tab)}
                >
                    {tab}
                    {#if tab === "headers" && headerCount > 0}
                        <span class="text-xs opacity-50">({headerCount})</span>
                    {/if}
                    {#if tab === "params" && hasParams}
                        <span
                            class="w-1.5 h-1.5 rounded-full bg-accent inline-block"
                        ></span>
                    {/if}
                    {#if tab === "auth" && request.auth.type !== "None"}
                        <span
                            class="w-1.5 h-1.5 rounded-full bg-accent inline-block"
                        ></span>
                    {/if}
                    {#if tab === "body" && bodyMode !== "none"}
                        <span
                            class="w-1.5 h-1.5 rounded-full bg-accent inline-block"
                        ></span>
                    {/if}
                </button>
            {/each}
        </div>
        <div class="border-b border-b-base-content/10"></div>

        <!-- tab content -->
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col">
            {#if activeTab === "params"}
                <RequestParamsTab
                    rows={paramRows}
                    onUpdateRow={(idx, field, val) =>
                        updateRow("params", idx, field, val)}
                    onRemoveRow={(idx) => removeRow("params", idx)}
                    onToggleRow={(idx) => toggleRow("params", idx)}
                />
            {:else if activeTab === "headers"}
                <RequestHeadersTab
                    rows={headerRows}
                    {showAutoHeaders}
                    {autoHeaderCount}
                    onUpdateRow={(idx, field, val) =>
                        updateRow("headers", idx, field, val)}
                    onRemoveRow={(idx) => removeRow("headers", idx)}
                    onToggleRow={(idx) => toggleRow("headers", idx)}
                    onToggleAutoHeaders={() =>
                        (showAutoHeaders = !showAutoHeaders)}
                />
            {:else if activeTab === "auth"}
                <RequestAuthTab
                    auth={request.auth}
                    onUpdate={(a) => emitUpdate({ auth: a })}
                />
            {:else if activeTab === "body"}
                <RequestBodyTab
                    {bodyMode}
                    {bodyType}
                    {rawBodyContent}
                    onBodyModeChange={(mode) => {
                        bodyMode = mode;
                        emitUpdate({
                            body: mode === "none" ? null : rawBodyContent,
                        });
                    }}
                    onBodyTypeChange={(type) => (bodyType = type)}
                    onRawBodyChange={(v) => {
                        rawBodyContent = v;
                        if (bodyMode === "raw") {
                            emitUpdate({ body: v });
                        }
                    }}
                    onBeautify={beautifyJson}
                />
            {/if}
        </div>
    </div>
</div>
