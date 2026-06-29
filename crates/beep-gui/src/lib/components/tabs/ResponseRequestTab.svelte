<script lang="ts">
    import type { SentRequest } from "$lib/types";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import { ChevronRightIcon } from "@lucide/svelte";

    interface Props {
        request: SentRequest;
        response: { status: number; elapsed_ms: number } | null; // Can be null if request return error before target return response.
    }

    let { request, response }: Props = $props();

    type AccordionSection = "general" | "headers" | "query" | "payload";
    let openSections = $state<Set<AccordionSection>>(new Set(["general", "headers", "query", "payload"]));

    function toggleSection(section: AccordionSection) {
        if (openSections.has(section)) {
            openSections.delete(section);
        } else {
            openSections.add(section);
        }
        openSections = new Set(openSections);
    }

    let queryViewMode = $state<"parsed" | "source">("parsed");

    function formatSize(bytes: number): string {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
    }

    let requestHeaders = $derived(
        request.headers?.map(([key, value]) => ({
            key, value, enabled: true, auto: false,
        })) ?? [],
    );

    let requestHeadersSize = $derived(request.size?.headers ?? 0);
    let requestBodySize = $derived(request.size?.body ?? 0);
    let requestBodyText = $derived(request.body ?? "");
    let protocol = $derived(request.http_version ?? "");

    function rawQueryString(url: string): string {
        const q = url.indexOf("?");
        return q < 0 ? "" : url.slice(q + 1);
    }

    function parseQueryParams(url: string): { key: string; value: string }[] {
        const raw = rawQueryString(url);
        return raw
            ? [...new URLSearchParams(raw).entries()].map(([key, value]) => ({ key, value }))
            : [];
    }

    let queryParams = $derived(parseQueryParams(request.url ?? ""));

    function queryStringText(): string {
        const raw = rawQueryString(request.url ?? "");
        return raw ? `?${raw}` : "";
    }
</script>

<div class="flex flex-col h-full min-h-0 overflow-auto">
    <!-- General -->
    <div class="border-b border-b-base-content/10">
        <button
            class="flex items-center gap-1 w-full px-1 py-1 text-xs font-semibold hover:bg-base-200 transition-colors cursor-pointer"
            onclick={() => toggleSection("general")}
        >
            <span class="transition-transform duration-150"
                class:rotate-90={openSections.has("general")}
            ><ChevronRightIcon class="w-3.5 h-3.5"></ChevronRightIcon></span>
            General
        </button>
        {#if openSections.has("general")}
            <div class="">
                <table class="table table-xs w-full table-fixed">
                    <tbody>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold w-72">Request URL</td>
                            <td class="text-xs font-mono">{request.url ?? ""}</td>
                        </tr>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold">Request Method</td>
                            <td class="text-xs font-mono">{request.method ?? ""}</td>
                        </tr>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold">Status Code</td>
                            <td class="text-xs font-mono"><StatusBadge status={response?.status || null} /></td>
                        </tr>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold">Protocol</td>
                            <td class="text-xs font-mono">{protocol}</td>
                        </tr>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold">Time</td>
                            <td class="text-xs font-mono">{response?.elapsed_ms} ms</td>
                        </tr>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold">Request Headers Size</td>
                            <td class="text-xs font-mono">{formatSize(requestHeadersSize)}</td>
                        </tr>
                        <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                            <td class="text-xs font-mono font-semibold">Request Body Size</td>
                            <td class="text-xs font-mono">{formatSize(requestBodySize)}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        {/if}
    </div>

    <!-- Request Headers -->
    <div class="border-b border-b-base-content/10">
        <button
            class="flex items-center gap-1 w-full px-1 py-1 text-xs font-semibold hover:bg-base-200 transition-colors cursor-pointer"
            onclick={() => toggleSection("headers")}
        >
            <span class="transition-transform duration-150"
                class:rotate-90={openSections.has("headers")}
            ><ChevronRightIcon class="w-3.5 h-3.5"></ChevronRightIcon></span>
            Request Headers
            <span class="text-xs opacity-50 font-normal">({requestHeaders.length})</span>
        </button>
        {#if openSections.has("headers")}
            <div class="pb-2">
                {#if requestHeaders.length > 0}
                    <table class="table table-xs table-pin-rows table-pin-cols w-full table-fixed">
                        <thead>
                            <tr>
                                <th class="text-xs w-72">Key</th>
                                <th class="text-xs">Value</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each requestHeaders as row, i (i)}
                                <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                                    <td class="align-top" title={row.key}>
                                        <span class="text-xs font-mono font-semibold break-all block">{row.key}</span>
                                    </td>
                                    <td class="align-top" title={row.value}>
                                        <span class="text-xs font-mono break-all block">{row.value}</span>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                {:else}
                    <div class="px-2 py-1 text-xs opacity-50 italic">No request headers</div>
                {/if}
            </div>
        {/if}
    </div>

    <!-- Query String Parameters -->
    {#if queryParams.length > 0}
        <div class="border-b border-b-base-content/10">
            <div
                class="flex items-center gap-1 w-full px-1 py-1 text-xs font-semibold hover:bg-base-200 transition-colors cursor-pointer"
                onclick={() => toggleSection("query")}
                role="button"
                tabindex="0"
                onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") toggleSection("query"); }}
            >
                <span class="transition-transform duration-150"
                    class:rotate-90={openSections.has("query")}
                ><ChevronRightIcon class="w-3.5 h-3.5"></ChevronRightIcon></span>
                Query String Parameters
                <span class="text-xs opacity-50 font-normal">({queryParams.length})</span>
                <button
                    class="btn btn-xs btn-ghost ml-auto"
                    class:btn-active={queryViewMode === "parsed"}
                    onclick={(e) => { e.stopPropagation(); queryViewMode = queryViewMode === "parsed" ? "source" : "parsed"; }}
                >view {queryViewMode === "parsed" ? "source" : "parsed"}</button>
            </div>
            {#if openSections.has("query")}
                <div class="pb-2">
                    {#if queryViewMode === "parsed"}
                        <table class="table table-xs table-pin-rows table-pin-cols w-full table-fixed">
                            <thead>
                                <tr>
                                    <th class="text-xs w-72">Key</th>
                                    <th class="text-xs">Value</th>
                                </tr>
                            </thead>
                            <tbody>
                                {#each queryParams as p, i (i)}
                                    <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                                        <td class="align-top" title={p.key}>
                                            <span class="text-xs font-mono font-semibold break-all block">{p.key}</span>
                                        </td>
                                        <td class="align-top" title={p.value}>
                                            <span class="text-xs font-mono break-all block">{p.value}</span>
                                        </td>
                                    </tr>
                                {/each}
                            </tbody>
                        </table>
                    {:else}
                        <pre class="text-xs font-mono px-2 py-1 bg-base-200 rounded whitespace-pre-wrap break-all select-text">{queryStringText()}</pre>
                    {/if}
                </div>
            {/if}
        </div>
    {/if}

    <!-- Request Payload -->
    {#if requestBodyText}
        <div class="border-b border-b-base-content/10">
            <button
                class="flex items-center gap-1 w-full px-1 py-1 text-xs font-semibold hover:bg-base-200 transition-colors cursor-pointer"
                onclick={() => toggleSection("payload")}
            >
                <span class="transition-transform duration-150"
                    class:rotate-90={openSections.has("payload")}
                ><ChevronRightIcon class="w-3.5 h-3.5"></ChevronRightIcon></span>
                Request Payload
                <span class="text-xs opacity-50 font-normal">({formatSize(requestBodySize)})</span>
            </button>
            {#if openSections.has("payload")}
                <div class="pb-2">
                    <pre class="text-xs font-mono px-2 py-1 bg-base-200 rounded whitespace-pre-wrap break-all select-text">{requestBodyText}</pre>
                </div>
            {/if}
        </div>
    {:else if queryParams.length === 0}
        <div class="border-b border-b-base-content/10">
            <div class="px-2 py-1 text-xs opacity-50 italic">No request payload</div>
        </div>
    {/if}
</div>
