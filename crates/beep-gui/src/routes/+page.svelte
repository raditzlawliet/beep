<script lang="ts">
    import type { HttpRequest } from "$lib/types";
    import { request, history, app, project } from "$lib/app-state.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open } from "@tauri-apps/plugin-dialog";
    import { listen } from "@tauri-apps/api/event";
    import { SvelteSet } from "svelte/reactivity";
    import TitleBar from "$lib/components/TitleBar.svelte";
    import StatusBar from "$lib/components/StatusBar.svelte";
    import RequestForm from "$lib/components/RequestForm.svelte";
    import ResponseView from "$lib/components/ResponseView.svelte";
    import HistorySidebar from "$lib/components/HistorySidebar.svelte";
    import ProjectSidebar from "$lib/components/ProjectSidebar.svelte";

    // local UI state (belongs to this page, not shared)

    let sidebarOpen = $state(false);
    let activePanel = $state<"history" | "project">("history");
    let activeFilePath = $state<string | null>(null);
    let expandedProject = new SvelteSet<string>();

    // request lifecycle
    let sending = $state(false);
    let reqError = $state<string | null>(null);

    // history lifecycle
    let histLoading = $state(false);
    let histError = $state<string | null>(null);

    // resizable vertical splitter (request/response)
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

    // resizable horizontal splitter (sidebar)
    let sidebarWidth = $state(260);
    let isDraggingSidebar = $state(false);

    const MIN_SIDEBAR = 180;
    const MAX_SIDEBAR = 480;

    function sidebarSplitterStart(e: MouseEvent) {
        isDraggingSidebar = true;
        e.preventDefault();
    }

    function sidebarSplitterMove(e: MouseEvent) {
        if (!isDraggingSidebar) return;
        let w = e.clientX;
        w = Math.max(MIN_SIDEBAR, Math.min(MAX_SIDEBAR, w));
        sidebarWidth = w;
    }

    function sidebarSplitterEnd() {
        isDraggingSidebar = false;
    }

    function toggleSidebar() {
        sidebarOpen = !sidebarOpen;
    }

    function switchToHistory() {
        if (activePanel === "history" && sidebarOpen) {
            sidebarOpen = false;
        } else {
            activePanel = "history";
            sidebarOpen = true;
        }
    }

    function switchToProject() {
        if (activePanel === "project" && sidebarOpen) {
            sidebarOpen = false;
        } else {
            activePanel = "project";
            sidebarOpen = true;
        }
    }

    async function handleOpenProject() {
        const selected = await open({ directory: true, multiple: false });
        if (selected && typeof selected === "string") {
            expandedProject.clear();
            activeFilePath = null;
            await project.open(selected);
            // auto-expand root
            if (project.tree.length > 0) {
                const rootPath = project.tree[0].path.replace(/[/\\][^/\\]*$/, '');
                expandedProject.add(rootPath);
            }
            activePanel = "project";
            sidebarOpen = true;
        }
    }

    function handleCloseProject() {
        project.close();
        expandedProject.clear();
        activeFilePath = null;
        if (activePanel === "project") {
            activePanel = "history";
            if (history.entries.length === 0) {
                sidebarOpen = false;
            }
        }
    }

    function toggleProjectDir(path: string) {
        if (expandedProject.has(path)) {
            expandedProject.delete(path);
        } else {
            expandedProject.add(path);
        }
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

    async function handleFileSelect(node: import("$lib/types").ProjectNode) {
        activeFilePath = node.path;
        try {
            const content = await invoke("read_file_content", { path: node.path });
            console.log(`--- ${node.name} ---`);
            console.log(content);
        } catch (e) {
            console.error(`Failed to read ${node.name}:`, e);
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

    // watch project directory for live tree updates
    $effect(() => {
        const p = project.path;
        if (!p) return;

        invoke("watch_project", { path: p });

        const unlisten = listen<{ parent_path: string; children: import("$lib/types").ProjectNode[] }>("fs-change", (event) => {
            // Do not remove; Debugging watcher
            const childNames = event.payload.children.map(c => c?.name ?? "null").join(", ");
            console.log(`[event/fs-change] ${event.payload.parent_path} : ${event.payload.children.length} children: ${childNames}`);

            project.applyNode(event.payload.parent_path, event.payload.children);
        });

        return () => {
            invoke("unwatch_project");
            unlisten.then((fn) => fn());
        };
    });

</script>

<div class="flex flex-col h-screen">
    <TitleBar
        onNewRequest={handleNewRequest}
        onOpenProject={handleOpenProject}
        onCloseProject={handleCloseProject}
        projectName={project.name}
    />

    <div class="flex flex-1 overflow-hidden">
        {#if sidebarOpen}
            <div
                class="shrink-0 overflow-hidden"
                style="width: {sidebarWidth}px"
            >
                {#if activePanel === "project"}
                    <ProjectSidebar
                        tree={project.tree}
                        projectName={project.name ?? ''}
                        activeFilePath={activeFilePath}
                        expanded={expandedProject}
                        onToggleDir={toggleProjectDir}
                        onFileSelect={handleFileSelect}
                        onOpenProject={handleOpenProject}
                    />
                {:else}
                    <HistorySidebar
                        entries={history.entries}
                        onSelect={handleHistorySelect}
                        onClearAll={handleClearHistory}
                        onDelete={handleDeleteHistory}
                    />
                {/if}
            </div>

            <!-- sidebar resize handle -->
            <div
                role="presentation"
                class="w-1 bg-base-300 hover:bg-primary cursor-col-resize shrink-0 transition-colors"
                class:bg-primary={isDraggingSidebar}
                onmousedown={sidebarSplitterStart}
            ></div>
        {/if}

        <div
            class="flex flex-col flex-1 overflow-hidden"
            bind:this={mainPanelEl}
            class:select-none={isDragging || isDraggingSidebar}
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

    <StatusBar
        onSwitchToHistory={switchToHistory}
        onSwitchToProject={switchToProject}
        activePanel={activePanel}
        sidebarOpen={sidebarOpen}
        hasProject={project.path !== null}
        response={request.response}
        loading={sending}
        error={reqError}
    />
</div>

<svelte:window onmousemove={(e) => { splitterMove(e); sidebarSplitterMove(e); }} onmouseup={() => { splitterEnd(); sidebarSplitterEnd(); }} />
