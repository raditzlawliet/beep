<script lang="ts">
    import type { ParsedRequest, HttpMethod, HttpVersion, HeaderField } from "$lib/types";
    import { methodTextColor } from "$lib/types";
    import { parseHttpVersion } from "$lib/http-file-utils";
    import { jsonrepair } from "jsonrepair";
    import { format } from "prettier/standalone";
    import * as htmlParser from "prettier/plugins/html";
    import xmlPlugin from "@prettier/plugin-xml";
    import RequestParamsTab from "$lib/components/tabs/RequestParamsTab.svelte";
    import RequestHeadersTab from "$lib/components/tabs/RequestHeadersTab.svelte";
    import RequestAuthTab from "$lib/components/tabs/RequestAuthTab.svelte";
    import RequestBodyTab from "$lib/components/tabs/RequestBodyTab.svelte";
    import RequestSettingsTab from "$lib/components/tabs/RequestSettingsTab.svelte";
    import { showToast } from "$lib/toast.svelte";

    interface Props {
        request: ParsedRequest;
        loading: boolean;
        onSend: (req: ParsedRequest) => void;
        onUpdate: (req: ParsedRequest) => void;
        defaultHeaders: [string, string][];
        initialTab?: string;
        onTabChange?: (tab: string) => void;
        onUrlBlur?: () => void;
    }

    let { request, loading, onSend, onUpdate, defaultHeaders, initialTab = "params", onTabChange, onUrlBlur }: Props = $props();

    type Tab = "params" | "headers" | "auth" | "body" | "settings";
    let activeTab = $state<Tab>("params");

    $effect(() => {
        activeTab = (initialTab as Tab) || "params";
    });

    export type BodyMode = "none" | "raw/json" | "raw/xml" | "raw/html" | "raw/text" | "form-urlencoded" | "form-multipart";
    let bodyMode = $state<BodyMode>("none");

    // Derived: raw body type extracted from combined bodyMode.
    const bodyType = $derived(
        bodyMode.startsWith("raw/") ? bodyMode.slice(4) : "text",
    );

    // Raw body editing state (CodeEditor needs local $state for reactivity).
    let rawBodyContent = $state("");

    const httpVersion = $derived(parseHttpVersion(request.http_version));

    function httpVersionToHttp(v: HttpVersion): string | null {
        if (v === "Http1") return "HTTP/1.1";
        if (v === "Http2") return "HTTP/2";
        return null;
    }

    // Sync rawBodyContent from request on mount / request switch.
    let _lastSyncedRawBody: string | null | undefined = $state(undefined);
    $effect(() => {
        const rb = request.body;
        bodyMode = (request.body_mode as BodyMode) ||
            (request.body ? "raw/text" : "none");

        // TODO not tested yet, since this review suggestion and the edge-case.
        // Editing raw body, then something triggers a re-parse that restores a slightly different version.
        if (rb === _lastSyncedRawBody) return;
        _lastSyncedRawBody = rb;
        rawBodyContent = rb ?? "";
    });

    function beautifyJson(): string {
        try {
            const repaired = jsonrepair(rawBodyContent);
            const parsed = JSON.parse(repaired);
            return JSON.stringify(parsed, null, 2);
        } catch (e) {
            showToast("Beautify failed", String(e));
            return rawBodyContent;
        }
    }

    async function beautifyHtml(): Promise<string> {
        try {
            return await format(rawBodyContent, {
                parser: "html",
                plugins: [htmlParser],
                tabWidth: 2,
            });
        } catch (e) {
            showToast("Beautify failed", String(e));
            return rawBodyContent;
        }
    }

    async function beautifyXml(): Promise<string> {
        try {
            return await format(rawBodyContent, {
                parser: "xml",
                plugins: [xmlPlugin],
                tabWidth: 2,
            });
        } catch (e) {
            showToast("Beautify failed", String(e));
            return rawBodyContent;
        }
    }

    async function beautify(): Promise<string> {
        if (bodyType === "json") return beautifyJson();
        if (bodyType === "html") return await beautifyHtml();
        if (bodyType === "xml") return await beautifyXml();
        return rawBodyContent;
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

    const isKnownMethod = $derived(METHODS.includes(request.method.toUpperCase() as HttpMethod));

    // Custom method combobox state
    let methodOpen = $state(false);
    let methodFilter = $state("");
    let methodTrigger = $state<HTMLElement | null>(null);

    function handleMethodSelect(m: string) {
        emitUpdate({ method: m });
        methodOpen = false;
        methodFilter = "";
    }

    function handleMethodInput(e: Event) {
        const v = (e.target as HTMLInputElement).value;
        methodFilter = v;
        emitUpdate({ method: v });
    }

    function handleMethodFocus() {
        methodFilter = request.method || "";
        methodOpen = true;
    }

    function handleMethodBlur() {
        // Delay close so click on option registers
        setTimeout(() => (methodOpen = false), 150);
    }

    // Tab badge indicators - derived directly from request data.
    const hasParams = $derived(request.query_params.filter((q) => q.enabled && q.key).length > 0);
    const headerCount = $derived(request.headers.filter((h) => h.enabled && h.key).length);
    const hasAuth = $derived(
        request.headers.some((h) => !h.auto && h.enabled && h.key.toLowerCase() === "authorization"),
    );

    // Merge auto-generated default headers into request so the tab badge
    // shows the correct count even before the headers tab is opened.
    $effect(() => {
        const existing = new Set(request.headers.map((h) => h.key.toLowerCase()));
        const merged: HeaderField[] = [...request.headers];
        let changed = false;
        for (const [k, v] of defaultHeaders) {
            if (!request.headers.some((h) => h.auto && h.key.toLowerCase() === k.toLowerCase())) {
                merged.push({ key: k, value: v, enabled: true, auto: true });
                changed = true;
            }
        }
        if (changed) {
            onUpdate({ ...request, headers: merged });
        }
    });

    function emitUpdate(overrides: Partial<ParsedRequest>) {
        onUpdate({ ...request, ...overrides });
    }

    function handleSend() {
        onSend({ ...request, body_mode: bodyMode,
            body: bodyMode.startsWith("raw/") ? rawBodyContent : request.body });
    }
</script>

<div class="card rounded-none bg-base-100 h-full">
    <div class="card-body p-0 flex flex-col min-h-0">
        <!-- method + url + send row -->
        <div class="join w-full p-2 pb-0">
            <div class="join-item relative w-28" bind:this={methodTrigger}>
                <input
                    type="text"
                    class="select select-bordered select-sm w-full font-mono font-bold {methodTextColor(request.method.toUpperCase() as HttpMethod)}"
                    value={request.method}
                    placeholder="METHOD"
                    onfocus={handleMethodFocus}
                    onblur={handleMethodBlur}
                    oninput={handleMethodInput}
                />
            </div>
            <input
                type="text"
                class="join-item input input-bordered input-sm flex-1 font-mono"
                placeholder="https://api.example.com/endpoint"
                value={request.url}
                oninput={(e) => {
                    emitUpdate({ url: (e.target as HTMLInputElement).value });
                }}
                onblur={() => onUrlBlur?.()}
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
        {#if methodOpen && methodTrigger}
            {@const rect = methodTrigger.getBoundingClientRect()}
            <ul
                class="fixed z-101 mt-1 bg-base-200 border border-base-content/20 rounded-box shadow-lg max-h-60 overflow-y-auto"
                style="top: {rect.bottom}px; left: {rect.left}px; width: {rect.width}px;"
            >
                {#each METHODS as m}
                    {@const selected = request.method.toUpperCase() === m}
                    <li>
                        <button
                            class="w-full text-left px-3 py-1 text-xs font-mono font-bold hover:bg-base-300 flex items-center gap-2 {selected ? 'bg-base-300' : ''} {methodTextColor(m)}"
                            onmousedown={() => handleMethodSelect(m)}
                        >
                            <span class="flex-1">{m}</span>
                            {#if selected}
                                <span class="text-xs opacity-50">✓</span>
                            {/if}
                        </button>
                    </li>
                {/each}
                <li>
                    <button
                        class="w-full text-left px-3 py-1 text-xs font-mono font-bold hover:bg-base-300 flex items-center gap-2 {!isKnownMethod ? 'bg-base-300' : ''}"
                        onmousedown={() => handleMethodSelect(isKnownMethod ? "OTHER" : request.method)}
                    >
                        <span class="flex-1">OTHER</span>
                        {#if !isKnownMethod}
                            <span class="text-xs opacity-50">✓</span>
                        {/if}
                    </button>
                </li>
            </ul>
        {/if}
        <div class="border-b border-b-base-content/10"></div>

        <!-- tabs -->
        <div role="tablist" class="tabs tabs-bordered tabs-xs px-1">
            {#each ["params", "auth", "headers", "body", "settings"] as tab}
                <button
                    role="tab"
                    class="tab capitalize gap-1.5 {activeTab === tab
                        ? 'tab-active'
                        : ''}"
                    onclick={() => { activeTab = tab as Tab; onTabChange?.(tab); }}
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
                    {#if tab === "auth" && hasAuth}
                        <span
                            class="w-1.5 h-1.5 rounded-full bg-accent inline-block"
                        ></span>
                    {/if}
                    {#if tab === "body" && bodyMode !== "none"}
                        <span
                            class="w-1.5 h-1.5 rounded-full bg-accent inline-block"
                        ></span>
                    {/if}
                    {#if tab === "settings" && httpVersion !== "Auto"}
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
                    initialValue={request.query_params}
                    url={request.url}
                    onchange={(params, displayUrl) => {
                        emitUpdate({ query_params: params, url: displayUrl });
                    }}
                />
            {:else if activeTab === "headers"}
                <RequestHeadersTab
                    initialValue={request.headers}
                    {defaultHeaders}
                    onchange={(headers) => {
                        emitUpdate({ headers });
                    }}
                    onFocusAuth={() => { activeTab = "auth"; onTabChange?.("auth"); }}
                />
            {:else if activeTab === "auth"}
                <RequestAuthTab
                    headers={request.headers}
                    onUpdate={(headers) => {
                        emitUpdate({ headers });
                    }}
                />
            {:else if activeTab === "body"}
                <RequestBodyTab
                    {bodyMode}
                    {rawBodyContent}
                    formUrlEncoded={request.form_urlencoded ?? []}
                    formMultipart={request.form_multipart ?? []}
                    onBodyModeChange={(mode) => {
                        bodyMode = mode;
                        emitUpdate({ body_mode: mode });
                    }}
                    onRawBodyChange={(v) => {
                        rawBodyContent = v;
                        emitUpdate({ body: v });
                    }}
                    onFormUrlEncodedChange={(fields) => {
                        emitUpdate({ form_urlencoded: fields });
                    }}
                    onFormMultipartChange={(fields) => {
                        emitUpdate({ form_multipart: fields });
                    }}
                    onBeautify={beautify}
                />
            {:else if activeTab === "settings"}
                <RequestSettingsTab
                    {httpVersion}
                    onUpdate={(v) => emitUpdate({ http_version: httpVersionToHttp(v) })}
                />
            {/if}
        </div>
    </div>
</div>
