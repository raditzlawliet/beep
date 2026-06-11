<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import type { HttpRequest, HttpResponse, HistoryEntry } from "$lib/types";
    import { defaultRequest } from "$lib/types";
    import TitleBar from "$lib/components/TitleBar.svelte";
    import StatusBar from "$lib/components/StatusBar.svelte";
    import RequestForm from "$lib/components/RequestForm.svelte";
    import ResponseView from "$lib/components/ResponseView.svelte";
    import HistorySidebar from "$lib/components/HistorySidebar.svelte";

    let request = $state<HttpRequest>(defaultRequest());
    let response = $state<HttpResponse | null>(null);
    let history = $state<HistoryEntry[]>([]);
    let loading = $state(false);
    let sidebarOpen = $state(false);
    let error = $state<string | null>(null);

    // --- resizable splitter ---
    let mainPanelEl = $state<HTMLDivElement | null>(null);
    let requestHeight = $state(300);
    let isDragging = $state(false);
    const MIN_REQUEST = 250;
    const MIN_RESPONSE = 200;

    function splitterStart(e: MouseEvent) {
        isDragging = true;
        e.preventDefault();
    }

    function splitterMove(e: MouseEvent) {
        if (!isDragging || !mainPanelEl) return;
        const rect = mainPanelEl.getBoundingClientRect();
        let h = e.clientY - rect.top;
        const maxH = rect.height - MIN_RESPONSE;
        h = Math.max(MIN_REQUEST, Math.min(maxH, h));
        requestHeight = h;
    }

    function splitterEnd() {
        isDragging = false;
    }

    async function handleSend(req: HttpRequest) {
        loading = true;
        error = null;
        try {
            const res = await invoke<HttpResponse>("execute_request", {
                payload: req,
            });
            response = res;
            const h = await invoke<HistoryEntry[]>("get_history");
            history = h;
        } catch (e) {
            error = String(e);
            response = null;
        } finally {
            loading = false;
        }
    }

    function handleUpdate(req: HttpRequest) {
        request = req;
    }

    function handleNewRequest() {
        request = defaultRequest();
        response = null;
        error = null;
    }

    function handleHistorySelect(entry: HistoryEntry) {
        request = { ...entry.request };
        response = entry.response ? { ...entry.response } : null;
        error = null;
    }

    function toggleSidebar() {
        sidebarOpen = !sidebarOpen;
    }

    $effect(() => {
        invoke<HistoryEntry[]>("get_history").then((h) => (history = h));
    });

    async function handleClearHistory() {
        await invoke("clear_history");
        history = await invoke<HistoryEntry[]>("get_history");
    }

    async function handleDeleteHistory(id: number) {
        await invoke("delete_history_entry", { id });
        history = await invoke<HistoryEntry[]>("get_history");
    }
</script>

<div class="flex flex-col h-screen">
    <TitleBar onNewRequest={handleNewRequest} />

    <div class="flex flex-1 overflow-hidden">
        {#if sidebarOpen}
            <div class="w-60 shrink-0">
                <HistorySidebar
                    entries={history}
                    onSelect={handleHistorySelect}
                    onClearAll={handleClearHistory}
                    onDelete={handleDeleteHistory}
                />
            </div>
        {/if}

        <div
            class="flex flex-col flex-1 overflow-hidden"
            bind:this={mainPanelEl}
            class:select-none={isDragging}
        >
            <div
                class="shrink-0 border-b border-base-300 overflow-hidden"
                style="height: {requestHeight}px"
            >
                <RequestForm
                    {request}
                    {loading}
                    onSend={handleSend}
                    onUpdate={handleUpdate}
                />
            </div>
            <div
                role="presentation"
                class="h-1 bg-base-300 hover:bg-primary cursor-row-resize shrink-0 transition-colors"
                class:bg-primary={isDragging}
                onmousedown={splitterStart}
            ></div>
            <div class="flex-1 overflow-hidden">
                <ResponseView {response} {loading} />
            </div>
        </div>
    </div>

    <StatusBar onToggleSidebar={toggleSidebar} {response} {loading} {error} />
</div>

<svelte:window onmousemove={splitterMove} onmouseup={splitterEnd} />
