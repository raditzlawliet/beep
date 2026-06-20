<script lang="ts">
    import type { HttpResponse } from "$lib/types";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import ResponseBodyTab from "$lib/components/tabs/ResponseBodyTab.svelte";
    import ResponseHeadersTab from "$lib/components/tabs/ResponseHeadersTab.svelte";
    import { UnplugIcon } from "@lucide/svelte";

    interface Props {
        response: HttpResponse | null;
        loading: boolean;
        error: string | null;
    }

    let { response, loading, error }: Props = $props();

    type Tab = "body" | "headers";
    let activeTab = $state<Tab>("body");

    function formatSize(bytes: number): string {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
    }
</script>

{#if loading}
    <progress class="progress progress-primary h-1 block"></progress>
{:else}
    <div class="h-1 block"></div>
{/if}
{#if response || error}
    <div class="card rounded-none bg-base-100 h-full relative">
        <div
            class="card-body flex flex-col min-h-0 p-0 gap-0"
            class:opacity-30={loading}
            class:pointer-events-none={loading}
        >
            <!-- Header bar: tabs + metadata -->
            <div class="flex items-end border-b border-base-300 py-1">
                <div role="tablist" class="tabs tabs-bordered tabs-xs px-1">
                    {#if response}
                        <button
                            role="tab"
                            class="tab"
                            class:tab-active={activeTab === "body"}
                            onclick={() => (activeTab = "body")}
                        >
                            Body
                        </button>
                        <button
                            role="tab"
                            class="tab gap-1.5"
                            class:tab-active={activeTab === "headers"}
                            onclick={() => (activeTab = "headers")}
                        >
                            Headers
                            <span class="text-xs opacity-50"
                                >({Object.keys(response.headers).length})</span
                            >
                        </button>
                    {:else}
                        <span class="tab tab-active">Response</span>
                    {/if}
                </div>
                <!-- Right metadata -->
                {#if response}
                    <div class="flex items-center gap-3 ml-auto pr-3 pb-1 text-sm">
                        <StatusBadge status={response.status} />
                        <span class="opacity-70 text-xs"
                            >{response.elapsed_ms}ms</span
                        >
                        <div class="dropdown dropdown-hover dropdown-end">
                            <span
                                tabindex="0"
                                role="button"
                                class="text-xs opacity-70 cursor-default hover:opacity-100 transition-opacity"
                            >
                                {formatSize(
                                    response.size.response_body +
                                        response.size.response_headers,
                                )}
                            </span>
                            <div
                                class="dropdown-content card card-compact bg-base-200 shadow-xl z-50 w-72 mt-1 p-3 border border-base-300 select-text"
                            >
                                <div
                                    class="text-xs font-semibold opacity-60 mb-2 uppercase tracking-wide"
                                >
                                    Response Size
                                </div>
                                <div class="flex flex-col gap-1">
                                    <div class="flex justify-between text-xs">
                                        <span class="opacity-60">Total</span>
                                        <span class="font-mono"
                                            >{formatSize(
                                                response.size.response_headers +
                                                    response.size.response_body,
                                            )}</span
                                        >
                                    </div>
                                    <div class="flex justify-between text-xs">
                                        <span class="opacity-60">Header</span>
                                        <span class="font-mono"
                                            >{formatSize(
                                                response.size.response_headers,
                                            )}</span
                                        >
                                    </div>
                                    <div class="flex justify-between text-xs">
                                        <span class="opacity-60">Body</span>
                                        <span class="font-mono"
                                            >{formatSize(
                                                response.size.response_body,
                                            )}</span
                                        >
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                {/if}
            </div>
            <!-- Content -->
            <div class="flex-1 min-h-0 overflow-auto">
                {#if response}
                    {#if activeTab === "body"}
                        <ResponseBodyTab {response} />
                    {:else}
                        <ResponseHeadersTab {response} />
                    {/if}
                {:else if error}
                    <div class="flex items-center justify-center h-full p-8">
                        <div class="flex flex-col items-center gap-2 max-w-lg text-center">
                            <UnplugIcon class="w-5 h-5 text-error" />
                            <p class="text-xs text-error">Could not send request</p>
                            <p class="text-xs text-base-content/60 whitespace-pre-wrap break-all font-mono">{error}</p>
                        </div>
                    </div>
                {/if}
            </div>
        </div>
    </div>
{:else}
    <div class="flex items-center justify-center h-full">
        <div class="flex flex-col items-center gap-2 opacity-30 text-sm">
            <span>Send a request to see the response</span>
        </div>
    </div>
{/if}
