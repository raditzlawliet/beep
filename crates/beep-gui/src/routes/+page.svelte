<script lang="ts">
    import type { HttpRequest } from "$lib/types";
    import { request, history, app } from "$lib/app-state.svelte";
    import TitleBar from "$lib/components/TitleBar.svelte";
    import StatusBar from "$lib/components/StatusBar.svelte";
    import RequestForm from "$lib/components/RequestForm.svelte";
    import ResponseView from "$lib/components/ResponseView.svelte";
    import HistorySidebar from "$lib/components/HistorySidebar.svelte";

    // local UI state (belongs to this page, not shared)

    let sidebarOpen = $state(false);

    // request lifecycle
    let sending = $state(false);
    let reqError = $state<string | null>(null);

    // history lifecycle
    let histLoading = $state(false);
    let histError = $state<string | null>(null);

    // resizable splitter
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

    function toggleSidebar() {
        sidebarOpen = !sidebarOpen;
    }

    // function wrapper
    async function handleSend(req: HttpRequest) {
        sending = true;
        reqError = null;
        try {
            await request.send(req);
        } catch (e) {
            reqError = typeof e === "string" ? e : (e as Error)?.message ?? String(e);
            history.refresh().catch(() => {});
        } finally {
            sending = false;
        }
    }

    async function handleClearHistory() {
        histLoading = true;
        histError = null;
        try {
            await history.clear();
        } catch (e) {
            histError = String(e);
        } finally {
            histLoading = false;
        }
    }

    async function handleDeleteHistory(id: number) {
        histLoading = true;
        histError = null;
        try {
            await history.delete(id);
        } catch (e) {
            histError = String(e);
        } finally {
            histLoading = false;
        }
    }

    function handleNewRequest() {
        request.reset();
        reqError = null;
    }

    async function handleHistorySelect(summary: import("$lib/types").HistoryEntrySummary) {
        try {
            await request.loadFromHistory(summary);
            reqError = summary.error ?? null;
        } catch (e) {
            reqError = typeof e === "string" ? e : (e as Error)?.message ?? String(e);
        }
    }

    // initialise
    $effect(() => {
        histLoading = true;
        histError = null;
        history.refresh()
            .catch((e) => (histError = String(e)))
            .finally(() => (histLoading = false));
    });

</script>

<div class="flex flex-col h-screen">
    <TitleBar onNewRequest={handleNewRequest} />

    <div class="flex flex-1 overflow-hidden">
        {#if sidebarOpen}
            <div class="w-60 shrink-0">
                <HistorySidebar
                    entries={history.entries}
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
                    request={request.current}
                    loading={sending}
                    onSend={handleSend}
                    onUpdate={request.update}
                    defaultHeaders={app.defaultHeaders}
                />
            </div>
            <div
                role="presentation"
                class="h-1 bg-base-300 hover:bg-primary cursor-row-resize shrink-0 transition-colors"
                class:bg-primary={isDragging}
                onmousedown={splitterStart}
            ></div>
            <div class="flex-1 overflow-hidden">
                <ResponseView response={request.response} loading={sending} error={reqError} />
            </div>
        </div>
    </div>

    <StatusBar onToggleSidebar={toggleSidebar} response={request.response} loading={sending} error={reqError} />
</div>

<svelte:window onmousemove={splitterMove} onmouseup={splitterEnd} />
