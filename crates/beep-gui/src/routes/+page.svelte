<script lang="ts">
    import type { HttpRequest, Tab, ProjectNode, ParsedRequest, HistoryEntrySummary } from "$lib/types";
    import { emptyParsedRequest, isHttpFile } from "$lib/types";
    import { request, history, app, project, httpFile } from "$lib/app-state.svelte";
    import { httpRequestToContent } from "$lib/http-file-utils";
    import { invoke } from "@tauri-apps/api/core";
    import { open, save } from "@tauri-apps/plugin-dialog";
    import { listen } from "@tauri-apps/api/event";
    import { SvelteSet } from "svelte/reactivity";
    import TitleBar from "$lib/components/TitleBar.svelte";
    import StatusBar from "$lib/components/StatusBar.svelte";
    import HistorySidebar from "$lib/components/HistorySidebar.svelte";
    import ProjectSidebar from "$lib/components/ProjectSidebar.svelte";
    import TabBar from "$lib/components/TabBar.svelte";
    import FileViewer from "$lib/components/FileViewer.svelte";
    import { PlusIcon, FolderOpenIcon } from "@lucide/svelte";
    import MainEditorViewer from "$lib/components/MainEditorViewer.svelte";

    // local UI state
    let sidebarOpen = $state(false);
    let activePanel = $state<"history" | "project">("history");
    let activeFilePath = $state<string | null>(null);
    let expandedProject = new SvelteSet<string>();

    let untitledCounter = $state(1);
    let historyTabCounter = $state(0);
    let tabs = $state<Tab[]>([]);
    let activeTabId = $state<string>("");

    // Tab helpers

    function isHttpTab(tab: Tab | undefined): boolean {
        if (!tab) return false;
        return tab.type === "http-file";
    }

    function findTab(id: string): Tab | undefined {
        return tabs.find((t) => t.id === id);
    }

    function createHttpTab(id: string, label: string, filePath?: string, persistent = true): Tab {
        return {
            id,
            type: "http-file",
            label,
            filePath,
            content: "",
            persistent,
            viewMode: "request",
            activeRequestIdx: 0,
            parsedRequests: [emptyParsedRequest()],
            fileVariables: [],
        };
    }

    function createUntitledTab(): Tab {
        const id = `__untitled_${untitledCounter}__`;
        untitledCounter++;
        const tab = createHttpTab(id, `Untitled-${untitledCounter - 1}.http`, undefined, true);
        tab.content = "";
        return tab;
    }

    // Open file tab
    function openFileTabPersistent(node: ProjectNode) {
        const existing = tabs.find((t) => t.id === node.path);
        if (existing) {
            if (!existing.persistent) existing.persistent = true;
            activeTabId = existing.id;
            return;
        }
        const isHttp = isHttpFile(node.name);
        const tab: Tab = isHttp
            ? createHttpTab(node.path, node.name, node.path, true)
            : {
                id: node.path,
                type: "file",
                label: node.name,
                filePath: node.path,
                content: "",
                persistent: true,
            };
        tabs.push(tab);
        activeTabId = node.path;
        loadFileContent(node.path);
    }

    function openFileTabTemp(node: ProjectNode) {
        const persistent = tabs.find((t) => t.id === node.path && t.persistent);
        if (persistent) {
            activeTabId = persistent.id;
            return;
        }
        const isHttp = isHttpFile(node.name);
        const tempIdx = tabs.findIndex((t) => !t.persistent);
        const tab: Tab = isHttp
            ? createHttpTab(node.path, node.name, node.path, false)
            : {
                id: node.path,
                type: "file",
                label: node.name,
                filePath: node.path,
                content: "",
                persistent: false,
            };
        if (tempIdx !== -1) {
            tabs[tempIdx] = tab;
        } else {
            tabs.push(tab);
        }
        activeTabId = node.path;
        loadFileContent(node.path);
    }

    async function loadFileContent(filePath: string) {
        try {
            const raw = await invoke<string>("read_file_content", { path: filePath });
            const tab = findTab(filePath);
            if (tab) {
                tab.content = raw;
                tab.originalContent = raw;
                tab.diskChanged = false;
                // For http files, the MainEditorViewer $effect will re-parse
            }
        } catch (e) {
            console.error(`Failed to read ${filePath}:`, e);
        }
    }

    // -- External change handler

    async function handleExternalChange(filePath: string) {
        try {
            const diskContent = await invoke<string>("read_file_content", { path: filePath });
            const tab = findTab(filePath);
            if (!tab) return;
            const hasEdits = tab.content !== tab.originalContent;
            if (!hasEdits) {
                tab.content = diskContent;
                tab.originalContent = diskContent;
                tab.diskChanged = false;
            } else {
                tab.diskChanged = true;
            }
        } catch (e) {
            console.error(`Failed to handle external change ${filePath}:`, e);
        }
    }

    // -- Save / Save All / Save As

    async function handleSave() {
        const tab = findTab(activeTabId);
        if (!tab) return;

        if (!tab.filePath) {
            // Untitled tab - Save As
            await handleSaveAs(tab);
            return;
        }

        await doSave(tab);
    }

    async function doSave(tab: Tab) {
        if (!tab.filePath || tab.content === undefined) return;
        try {
            await invoke("save_file_content", { path: tab.filePath, content: tab.content });
            tab.originalContent = tab.content;
            tab.diskChanged = false;
        } catch (e) {
            console.error(`Failed to save ${tab.filePath}:`, e);
        }
    }

    async function handleSaveAs(tab: Tab) {
        const ext = tab.type === "http-file" ? "http" : "json";
        const selected = await save({
            filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
            defaultPath: tab.label,
        });
        if (!selected || typeof selected !== "string") return;

        // Rename the tab
        const oldId = tab.id;
        tab.id = selected;
        tab.filePath = selected;
        tab.label = selected.split(/[/\\]/).pop() || selected;

        // Update tabs array references
        if (activeTabId === oldId) activeTabId = selected;

        await doSave(tab);
    }

    async function handleSaveAll() {
        for (const tab of tabs) {
            if (!tab.filePath || tab.content === undefined) continue;
            if (tab.content === tab.originalContent && !tab.diskChanged) continue;
            await doSave(tab);
        }
    }

    function hasUnsavedTabs(): boolean {
        return tabs.some((t) =>
            t.originalContent !== undefined && t.content !== t.originalContent
        );
    }

    // -- Close tab

    function closeTab(id: string) {
        const idx = tabs.findIndex((t) => t.id === id);
        if (idx === -1) return;
        // TODO: dirty confirmation dialog
        tabs.splice(idx, 1);
        if (activeTabId === id) {
            if (tabs.length === 0) {
                activeTabId = "";
            } else if (idx < tabs.length) {
                activeTabId = tabs[idx].id;
            } else {
                activeTabId = tabs[tabs.length - 1].id;
            }
        }
    }

    function handleCloseTab() {
        closeTab(activeTabId);
    }

    function selectTab(id: string) {
        activeTabId = id;
    }

    function makeTabPersistent(id: string) {
        const tab = findTab(id);
        if (tab && !tab.persistent) tab.persistent = true;
    }

    // -- New Request flows

    function handleNewUntitled() {
        request.reset();
        reqError = null;
        const tab = createUntitledTab();
        tabs.push(tab);
        activeTabId = tab.id;
    }

    async function handleNewInFile() {
        const activeTab = findTab(activeTabId);
        if (!activeTab || !isHttpTab(activeTab)) return;

        request.reset();
        reqError = null;

        const newReq: ParsedRequest = {
            ...emptyParsedRequest(),
            title: "New Request",
        };

        try {
            const newContent = await httpFile.appendRequest(activeTab.content, newReq);
            activeTab.content = newContent;
            activeTab.persistent = true;

            // Switch to request mode and select the new request
            activeTab.viewMode = "request";
            // The MainEditorViewer $effect will re-parse and update indices
        } catch (e) {
            console.error("Failed to append request:", e);
        }
    }

    function handleNewRequest() {
        // Smart Mode: in-file if active tab is http, else new untitled
        const activeTab = findTab(activeTabId);
        if (activeTab && isHttpTab(activeTab)) {
            handleNewInFile();
        } else {
            handleNewUntitled();
        }
    }

    // -- Content change (from MainEditorViewer or FileViewer)

    function handleContentChange(newContent: string) {
        const tab = findTab(activeTabId);
        if (!tab) return;
        tab.content = newContent;
        if (!tab.persistent && tab.originalContent !== undefined && newContent !== tab.originalContent) {
            tab.persistent = true;
        }
    }

    function handleTabStateChange(state: Partial<Tab>) {
        const tab = findTab(activeTabId);
        if (!tab) return;
        Object.assign(tab, state);
    }

    // -- Request lifecycle

    let sending = $state(false);
    let reqError = $state<string | null>(null);

    // -- History lifecycle (TODO)

    let histLoading = $state(false);
    let histError = $state<string | null>(null);

    // -- Resizable splitter

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

    // -- Sidebar splitter

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

    // -- Project

    async function handleOpenProject() {
        const selected = await open({ directory: true, multiple: false });
        if (selected && typeof selected === "string") {
            if (project.path && project.path !== selected) {
                // Close all file-backed tabs
                tabs = tabs.filter((t) => !t.filePath?.startsWith(project.path!));
                if (!findTab(activeTabId)) activeTabId = tabs[0]?.id ?? "";
            }
            expandedProject.clear();
            activeFilePath = null;
            await project.open(selected);

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
        // Keep only untitled tabs (no filePath)
        tabs = tabs.filter((t) => !t.filePath);
        if (!findTab(activeTabId)) activeTabId = tabs[0]?.id ?? "";
        if (activePanel === "project") {
            activePanel = "history";
            if (history.entries.length === 0) sidebarOpen = false;
        }
    }

    async function toggleProjectDir(path: string) {
        if (expandedProject.has(path)) {
            expandedProject.delete(path);
        } else {
            expandedProject.add(path);
            if (!project.isLoaded(path)) {
                await project.expand(path);
            }
        }
    }

    // -- Request / Send

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

    // -- History

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

    async function handleHistorySelect(summary: HistoryEntrySummary) {
        await openHistoryEntry(summary, false);
    }

    async function handleHistoryDblClick(summary: HistoryEntrySummary) {
        await openHistoryEntry(summary, true);
    }

    async function openHistoryEntry(summary: HistoryEntrySummary, persistent: boolean) {
        try {
            await request.loadFromHistory(summary);
            reqError = summary.error ?? null;
            historyTabCounter++;
            const id = `__history_${historyTabCounter}__`;

            // Temp clicks: replace the single temp slot
            if (!persistent) {
                const tempIdx = tabs.findIndex((t) => !t.persistent);
                if (tempIdx !== -1) tabs.splice(tempIdx, 1);
            }

            const content = httpRequestToContent(request.current);
            tabs.push({
                id, type: "http-file" as const,
                label: `${summary.method} ${summary.url.slice(0, 50)}.http`,
                content, originalContent: content, persistent, diskChanged: false,
                viewMode: "request", activeRequestIdx: 0,
                parsedRequests: [], fileVariables: [],
            });
            activeTabId = id;
        } catch (e) {
            reqError = typeof e === "string" ? e : (e as Error)?.message ?? String(e);
        }
    }

    function handleFileSelect(node: ProjectNode) {
        activeFilePath = node.path;
        openFileTabTemp(node);
    }

    function handleFileDblClick(node: ProjectNode) {
        activeFilePath = node.path;
        openFileTabPersistent(node);
    }

    // -- Initialise

    $effect(() => {
        histLoading = true;
        histError = null;
        history.refresh()
            .catch((e) => (histError = String(e)))
            .finally(() => (histLoading = false));
    });

    // -- Watch project

    $effect(() => {
        const p = project.path;
        if (!p) return;
        invoke("watch_project", { path: p });
        const unlistenChange = listen<{ parent_path: string; children: ProjectNode[] }>("fs-change", (event) => {
            project.applyNode(event.payload.parent_path, event.payload.children);
        });
        const unlistenContent = listen<{ path: string }>("fs-content-change", (event) => {
            handleExternalChange(event.payload.path);
        });
        return () => {
            invoke("unwatch_project");
            unlistenChange.then((fn) => fn());
            unlistenContent.then((fn) => fn());
        };
    });

    // -- Derived

    let activeTab = $derived(findTab(activeTabId));
    let activeFileName = $derived(activeTab?.label ?? null);
    let hasActiveHttpTab = $derived(isHttpTab(activeTab));
    let hasActiveFileTab = $derived(activeTab?.type === "file");

    $effect(() => {
        const tab = activeTab;
        if (tab?.filePath) {
            activeFilePath = tab.filePath;
            let parent = tab.filePath.replace(/[/\\][^/\\]*$/, "");
            while (parent && parent !== project.path) {
                expandedProject.add(parent);
                const next = parent.replace(/[/\\][^/\\]*$/, "");
                if (next === parent) break;
                parent = next;
            }
        }
    });
</script>

<div class="flex flex-col h-screen">
    <TitleBar
        {activeFileName}
        projectName={project.name}
        {hasActiveHttpTab}
        {hasActiveFileTab}
        hasUnsavedTabs={hasUnsavedTabs()}
        onNewUntitled={handleNewUntitled}
        onNewInFile={handleNewInFile}
        onNewRequest={handleNewRequest}
        onOpenProject={handleOpenProject}
        onCloseProject={handleCloseProject}
        onCloseTab={handleCloseTab}
        onSave={handleSave}
        onSaveAll={handleSaveAll}
    />

    <div class="flex flex-1 overflow-hidden">
        {#if sidebarOpen}
            <div class="shrink-0 overflow-hidden" style="width: {sidebarWidth}px">
                {#if activePanel === "project"}
                    <ProjectSidebar
                        tree={project.tree}
                        projectName={project.name ?? ''}
                        {activeFilePath}
                        expanded={expandedProject}
                        loadingDirs={project.loadingDirs}
                        onToggleDir={toggleProjectDir}
                        onFileSelect={handleFileSelect}
                        onFileDblClick={handleFileDblClick}
                        onOpenProject={handleOpenProject}
                    />
                {:else}
                    <HistorySidebar
                        entries={history.entries}
                        onSelect={handleHistorySelect}
                        onDblClick={handleHistoryDblClick}
                        onClearAll={handleClearHistory}
                        onDelete={handleDeleteHistory}
                    />
                {/if}
            </div>

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
            {#if tabs.length > 0}
                <TabBar
                    {tabs}
                    {activeTabId}
                    onSelectTab={selectTab}
                    onCloseTab={closeTab}
                    onDblClickTab={makeTabPersistent}
                />
            {/if}

            {#if activeTab?.type === "http-file"}
                <MainEditorViewer
                    tab={activeTab}
                    {sending}
                    {reqError}
                    response={request.response}
                    onContentChange={handleContentChange}
                    onTabStateChange={handleTabStateChange}
                    onSend={handleSend}
                    {requestHeight}
                    onSplitterStart={splitterStart}
                />
            {:else if activeTab?.type === "file"}
                <div class="flex-1 min-h-0 overflow-hidden">
                    <FileViewer
                        fileName={activeTab.label}
                        content={activeTab.content ?? ""}
                        initialCursorPos={activeTab.cursorPos}
                        oncursorchange={(pos) => {
                            if (activeTab) activeTab.cursorPos = pos;
                        }}
                        onContentChange={handleContentChange}
                    />
                </div>
            {:else if !project.path}
                <!-- No project + no tab - welcome state -->
                <div class="flex-1 flex items-center justify-center">
                    <div class="flex flex-col gap-6 w-80 select-none">
                        <!-- Header: icon + title -->
                        <div class="flex items-center justify-center gap-4">
                            <img src="/favicon.png" alt="Beep" class="w-10 h-10 opacity-80 shrink-0" />
                            <div>
                                <div class="text-lg">Welcome back to Beep</div>
                                <div class="text-sm italic font-thin text-neutral-content">The next intuitive API client</div>
                            </div>
                        </div>

                        <!-- Get Started -->
                        <div>
                            <div class="divider divider-start text-xs text-neutral-content uppercase m-0 my-2">Get Started</div>
                            <div class="flex flex-col">
                                <button class="btn btn-xs btn-ghost justify-start gap-2 w-full h-8" onclick={handleNewRequest}>
                                    <PlusIcon class="w-3.5 h-3.5 text-neutral-content" />
                                    <span class="flex-1 text-start font-normal">New Request</span>
                                    <span class="text-neutral-content text-xs hidden">Ctrl N</span>
                                </button>
                                <button class="btn btn-xs btn-ghost justify-start gap-2 w-full h-8" onclick={handleOpenProject}>
                                    <FolderOpenIcon class="w-3.5 h-3.5 text-neutral-content" />
                                    <span class="flex-1 text-start font-normal">Open Project</span>
                                    <span class="text-neutral-content text-xs hidden">Ctrl O</span>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            {/if}
        </div>
    </div>

    <StatusBar
        onSwitchToHistory={switchToHistory}
        onSwitchToProject={switchToProject}
        {activePanel}
        {sidebarOpen}
        hasProject={project.path !== null}
        response={request.response}
        loading={sending}
        error={reqError}
    />
</div>

<svelte:window
    onmousemove={(e) => { splitterMove(e); sidebarSplitterMove(e); }}
    onmouseup={() => { splitterEnd(); sidebarSplitterEnd(); }}
/>
