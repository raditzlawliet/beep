<script lang="ts">
    import type { HttpRequest, HttpMethod } from "$lib/types";
    import { methodTextColor } from "$lib/types";
    import { jsonrepair } from "jsonrepair";
    import { format } from "prettier/standalone";
    import * as htmlParser from "prettier/plugins/html";
    import xmlPlugin from "@prettier/plugin-xml";
    import RequestParamsTab from "$lib/components/tabs/RequestParamsTab.svelte";
    import RequestHeadersTab from "$lib/components/tabs/RequestHeadersTab.svelte";
    import RequestAuthTab from "$lib/components/tabs/RequestAuthTab.svelte";
    import RequestBodyTab from "$lib/components/tabs/RequestBodyTab.svelte";
    import { showToast } from "$lib/toast.svelte";

    interface Props {
        request: HttpRequest;
        loading: boolean;
        onSend: (req: HttpRequest) => void;
        onUpdate: (req: HttpRequest) => void;
        defaultHeaders: [string, string][];
    }

    let { request, loading, onSend, onUpdate, defaultHeaders }: Props = $props();

    type Tab = "params" | "headers" | "auth" | "body";
    let activeTab = $state<Tab>("params");

    export type BodyMode = "none" | "raw/json" | "raw/xml" | "raw/html" | "raw/text" | "form-urlencoded" | "form-multipart";
    let bodyMode = $state<BodyMode>("none");

    // Derived: raw body type extracted from combined bodyMode.
    const bodyType = $derived(
        bodyMode.startsWith("raw/") ? bodyMode.slice(4) : "text",
    );

    // Raw body editing state (CodeEditor needs local $state for reactivity).
    let rawBodyContent = $state("");

    // Sync rawBodyContent from request on mount / history load.
    let _rawSnap: string | null | undefined = $state(undefined);
    $effect(() => {
        const rb = request.raw_body;
        if (rb === _rawSnap) return;
        _rawSnap = rb;
        rawBodyContent = rb ?? "";
        bodyMode = (request.body_mode as BodyMode) ||
            (request.raw_body ? "raw/text" : "none");
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

    // Tab badge indicators - derived directly from request data.
    const hasParams = $derived(Object.keys(request.query_params).length > 0);
    const headerCount = $derived(Object.keys(request.headers).length);

    function emitUpdate(overrides: Partial<HttpRequest>) {
        onUpdate({ ...request, ...overrides });
    }

    function handleSend() {
        onSend({ ...request, body_mode: bodyMode,
            raw_body: bodyMode.startsWith("raw/") ? rawBodyContent : request.raw_body });
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
                    emitUpdate({ url: (e.target as HTMLInputElement).value });
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
                    initialValue={request.query_params}
                    url={request.url}
                    onchange={(params, newUrl) => {
                        emitUpdate({ query_params: params, url: newUrl });
                    }}
                />
            {:else if activeTab === "headers"}
                <RequestHeadersTab
                    initialValue={request.headers}
                    {defaultHeaders}
                    onchange={(headers) => {
                        emitUpdate({ headers });
                    }}
                />
            {:else if activeTab === "auth"}
                <RequestAuthTab
                    auth={request.auth}
                    onUpdate={(a) => emitUpdate({ auth: a })}
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
                        emitUpdate({ raw_body: v });
                    }}
                    onFormUrlEncodedChange={(fields) => {
                        emitUpdate({ form_urlencoded: fields });
                    }}
                    onFormMultipartChange={(fields) => {
                        emitUpdate({ form_multipart: fields });
                    }}
                    onBeautify={beautify}
                />
            {/if}
        </div>
    </div>
</div>
