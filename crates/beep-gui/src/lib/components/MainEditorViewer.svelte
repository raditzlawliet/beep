<script lang="ts">
    import type {
        Tab,
        ParsedRequest,
        ParsedFileVariable,
        ViewMode,
        HttpResult,
    } from "$lib/types";
    import { emptyParsedRequest } from "$lib/types";
    import { app, httpFile, project } from "$lib/app-state.svelte";
    import { parsedToFormRequest, formRequestToParsed } from "$lib/http-file-utils";
    import MainEditorToolbar from "$lib/components/MainEditorToolbar.svelte";
    import FileEditor from "$lib/components/FileEditor.svelte";
    import RequestForm from "$lib/components/RequestForm.svelte";
    import ResponseView from "$lib/components/ResponseView.svelte";
    import FileOverview from "$lib/components/FileOverview.svelte";
    import { setSendHandler, setModeHandler } from "$lib/hotkeys.svelte";

    interface Props {
        tab: Tab;
        sending: boolean;
        reqError: string | null;
        result: HttpResult | null;
        onContentChange: (newContent: string) => void;
        onTabStateChange: (state: Partial<Tab>) => void;
        onSend: (req: ParsedRequest, fileVars: ParsedFileVariable[]) => void;
        requestHeight: number;
        onSplitterStart: (e: MouseEvent) => void;
    }

    let {
        tab,
        sending,
        reqError,
        result,
        onContentChange,
        onTabStateChange,
        onSend,
        requestHeight,
        onSplitterStart,
    }: Props = $props();

    let parsedRequests = $state<ParsedRequest[]>([]);
    let fileVariables = $state<ParsedFileVariable[]>([]);
    let activeRequestIdx = $state(0);
    let viewMode = $state<ViewMode>("request");
    let cursorPos = $state<number | undefined>(undefined);
    let formRequest = $state<ParsedRequest>(parsedToFormRequest(undefined));
    let requestFormTab = $state<string>("params");
    let fileOverviewTab = $state<string>("requests");
    let editorLanguage = $derived(detectLanguage(tab.filePath ?? "", tab.content));

    function detectLanguage(path: string, _raw: string): "text" | "http" {
        const ext = path.split(".").pop()?.toLowerCase();
        if (ext === "http" || ext === "rest") return "http";
        return "text";
    }

    // Run gutter dropdown
    let runDropdown = $state<{ requestIdx: number; x: number; y: number } | null>(null);
    let runMarkers = $derived(parsedRequests.map((r) => r.block_region.start));

    // Parse generation counter to discard stale async results.
    let _parseGen = $state(0);
    let _lastParsedContent = $state("");

    function saveTabState(state: Partial<Tab>) {
        onTabStateChange(state);
    }

    // Sync from tab on identity change only
    let _lastTabId = $state("");
    $effect(() => {
        if (tab.id === _lastTabId) return;
        _lastTabId = tab.id;
        parsedRequests = tab.parsedRequests ?? [];
        fileVariables = tab.fileVariables ?? [];
        activeRequestIdx = tab.activeRequestIdx ?? 0;
        viewMode = tab.viewMode ?? "request";
        cursorPos = tab.cursorPos;
        requestFormTab = tab.requestFormTab ?? "params";
        fileOverviewTab = tab.fileOverviewTab ?? "requests";
        formRequest = parsedToFormRequest(parsedRequests[activeRequestIdx]);
    });

    // Parse content when it changes
    $effect(() => {
        const content = tab.content;

        // Skip if content hasn't actually changed since last parse.
        if (content === _lastParsedContent) return;
        _lastParsedContent = content ?? "";

        if (!content && parsedRequests.length === 0) {
            viewMode = "request";
            activeRequestIdx = 0;
            parsedRequests = [emptyParsedRequest()];
            fileVariables = [];
            formRequest = parsedToFormRequest(parsedRequests[0]);
            saveTabState({ viewMode, activeRequestIdx: 0, parsedRequests, fileVariables });
            return;
        }
        if (!content) return;

        const gen = ++_parseGen;
        httpFile.parse(content).then((result) => {
            // Discard stale results from a previous parse.
            if (gen !== _parseGen) return;
            parsedRequests = result.requests;
            fileVariables = result.variables;
            if (parsedRequests.length === 0) {
                parsedRequests = [emptyParsedRequest()];
                activeRequestIdx = 0;
            } else if (activeRequestIdx >= parsedRequests.length) {
                activeRequestIdx = Math.max(0, parsedRequests.length - 1);
            }

            // Populate form from fresh parse.
            if (viewMode === "request") {
                const parsed = parsedToFormRequest(parsedRequests[activeRequestIdx]);
                // Restore display URL: inline params must show in the URL field.
                const inline = (parsed.query_params ?? []).filter((q) => q.is_inline && q.enabled && q.key);
                if (inline.length > 0) {
                    const qs = inline.map((q) => `${encodeURIComponent(q.key)}=${encodeURIComponent(q.value)}`).join("&");
                    parsed.url = `${parsed.url}?${qs}`;
                }
                formRequest = parsed;
            }
            saveTabState({ parsedRequests, fileVariables, activeRequestIdx });
        }).catch((e) => console.error("Failed to parse http file:", e));
    });

    function handleCodeChange(newContent: string) {
        onContentChange(newContent);
    }

    function handleCursorChange(pos: number) {
        cursorPos = pos;
        saveTabState({ cursorPos });
        const content = tab.content;
        if (!content) return;
        let found = -1;
        for (let i = 0; i < parsedRequests.length; i++) {
            if (pos >= parsedRequests[i].block_region.start && pos <= parsedRequests[i].block_region.end + 1) {
                found = i; break;
            }
        }
        if (found === -1 && parsedRequests.length > 0) {
            found = pos < (parsedRequests[0]?.block_region.start ?? 0) ? 0 : parsedRequests.length - 1;
        }
        if (found !== -1 && found !== activeRequestIdx) {
            activeRequestIdx = found;
            saveTabState({ activeRequestIdx: found });
            formRequest = parsedToFormRequest(parsedRequests[found]);
        }
    }

    function handleSelectRequest(idx: number) {
        activeRequestIdx = idx;
        saveTabState({ activeRequestIdx: idx });
        if (idx >= 0 && idx < parsedRequests.length) {
            formRequest = parsedToFormRequest(parsedRequests[idx]);
        }

        // Reposition cursor to selected request's start and focus editor in code mode
        if (idx >= 0 && idx < parsedRequests.length) {
            cursorPos = parsedRequests[idx].block_region.start;
            saveTabState({ cursorPos });

            // set focus code editor if in code mode
            if (viewMode === "code") {
                requestAnimationFrame(() => {
                    const cm = document.querySelector('.cm-content') as HTMLElement | null;
                    cm?.focus();
                });
            }
        }
    }

    function handleSetMode(mode: ViewMode) {
        if (viewMode === mode) return;
        // sync before leaving request mode
        if (viewMode === "request" && mode !== "request") {
            syncFormToContent();
        }
        viewMode = mode;
        if (mode === "request") {
            formRequest = parsedToFormRequest(parsedRequests[activeRequestIdx]);
        } else if (mode === "code") {
            // Reposition cursor to active request's start if cursor is outside its scope
            const req = parsedRequests[activeRequestIdx];
            if (req) {
                const within = cursorPos !== undefined
                    && cursorPos >= req.block_region.start
                    && cursorPos <= req.block_region.end + 1;
                if (!within) {
                    cursorPos = req.block_region.start;
                    saveTabState({ cursorPos });
                }
            }
        }
        saveTabState({ viewMode });
    }

    function handleRequestFormTabChange(tabName: string) {
        requestFormTab = tabName;
        saveTabState({ requestFormTab: tabName });
    }

    function handleFileOverviewTabChange(tabName: string) {
        fileOverviewTab = tabName;
        saveTabState({ fileOverviewTab: tabName });
    }

    function handleNavigateToRequest(idx: number) {
        activeRequestIdx = idx;
        viewMode = "request";
        formRequest = parsedToFormRequest(parsedRequests[idx]);
        saveTabState({ viewMode, activeRequestIdx: idx });
    }

async function handleVariablesUpdate(vars: ParsedFileVariable[]) {
    fileVariables = vars;
    saveTabState({ fileVariables: vars });
        try {
            const newContent = await httpFile.updateVars(tab.content, vars);
            onContentChange(newContent);
        } catch (e) {
            console.error("Failed to update variables:", e);
        }
    }

    async function syncFormToContent() {
        const base = parsedRequests[activeRequestIdx];
        if (!base) return;
        const updated = formRequestToParsed(formRequest, base);
        parsedRequests[activeRequestIdx] = updated;
        saveTabState({ parsedRequests });
        try {
            const newContent = await httpFile.updateRequest(tab.content, activeRequestIdx, updated);
            // Avoid re-triggering parse/loop when content hasn't actually changed.
            if (newContent !== tab.content) {
                onContentChange(newContent);
            }
        } catch (e) {
            console.error("Failed to sync form to content:", e);
        }
    }

    function handleFormUpdate(req: ParsedRequest) {
        formRequest = req;
        const base = parsedRequests[activeRequestIdx];
        if (!base) return;
        const updated = formRequestToParsed(req, base);
        parsedRequests[activeRequestIdx] = updated;
        saveTabState({ parsedRequests });
    }

    function handleSend(req: ParsedRequest) {
        // Only sync form to content when in request mode (form may have been edited).
        // In code/file mode, formRequest already reflects the active parsed request.
        if (viewMode === "request") {
            syncFormToContent();
        }

        // Switch to request mode to show result when sent from code/file mode
        if (viewMode !== "request") {
            viewMode = "request";
            formRequest = parsedToFormRequest(parsedRequests[activeRequestIdx]);
            saveTabState({ viewMode });
        }
        onSend(req, fileVariables ?? []);
    }

    // --- Run gutter ---

    function handleRunMarkerClick(pos: number, event: MouseEvent) {
        const idx = parsedRequests.findIndex((r) => r.block_region.start === pos);
        if (idx === -1) return;
        runDropdown = { requestIdx: idx, x: event.clientX, y: event.clientY };
    }

    function sendRunRequest(idx: number) {
        handleSelectRequest(idx);
        runDropdown = null;
        // Switch to request mode to show result
        if (viewMode !== "request") {
            viewMode = "request";
            saveTabState({ viewMode });
        }
        if (idx >= 0 && idx < parsedRequests.length) {
            onSend(parsedToFormRequest(parsedRequests[idx]), fileVariables ?? []);
        }
    }

    function closeRunDropdown() {
        runDropdown = null;
    }

    function handleUrlBlur() {
        const url = formRequest.url;
        const qIdx = url.indexOf('?');

        // Parse URL query into key-value pairs
        const urlParams: { key: string; value: string }[] = [];
        if (qIdx >= 0) {
            for (const part of url.slice(qIdx + 1).split('&')) {
                const eq = part.indexOf('=');
                if (eq >= 0) {
                    urlParams.push({ key: safeDecodeURI(part.slice(0, eq)), value: safeDecodeURI(part.slice(eq + 1)) });
                } else if (part.trim()) {
                    urlParams.push({ key: safeDecodeURI(part), value: '' });
                }
            }
        }

        // Merge: keep multiline params, replace inline with URL query
        const merged = formRequest.query_params
            .filter((q) => !q.is_inline && q.key) // keep multiline
            .map((q) => ({ ...q }));
        const urlKeys = new Set(urlParams.map((p) => p.key));
        for (const p of urlParams) {
            if (p.key) merged.push({ key: p.key, value: p.value, enabled: true, is_inline: true });
        }

        // Clean URL: strip query
        const cleanUrl = qIdx >= 0 ? url.slice(0, qIdx) : url;
        formRequest = { ...formRequest, url: cleanUrl, query_params: merged };
        syncFormToContent().catch(console.error);
    }

    function safeDecodeURI(s: string): string {
        try { return decodeURIComponent(s); } catch { return s; }
    }

    let fileName = $derived(
        tab.filePath && project.path && tab.filePath.startsWith(project.path)
            ? tab.filePath.slice(project.path.length).replace(/^[/\\]/, "")
            : tab.label
    );

    // Register Ctrl+Enter send handler and Ctrl+1/2/3 mode handler with the hotkey system
    $effect(() => {
        setSendHandler(() => handleSend(formRequest));
        setModeHandler((mode: string) => handleSetMode(mode as ViewMode));
        return () => {
            setSendHandler(null);
            setModeHandler(null);
        };
    });
</script>

<div class="flex flex-col h-full min-h-0">
    <MainEditorToolbar
        {fileName}
        tabType="http-file"
        requests={parsedRequests}
        {activeRequestIdx}
        {viewMode}
        onSelectRequest={handleSelectRequest}
        onSetMode={handleSetMode}
        onSend={() => handleSend(formRequest)}
    />

    {#if viewMode === "code"}
        <div class="flex-1 min-h-0 overflow-hidden">
            <FileEditor
                value={tab.content}
                language={editorLanguage}
                wrapLines={true}
                onchange={handleCodeChange}
                initialCursorPos={cursorPos}
                oncursorchange={handleCursorChange}
                runMarkers={runMarkers}
                onRunMarkerClick={handleRunMarkerClick}
                class="h-full"
            />
        </div>

        <!-- Run dropdown -->
        {#if runDropdown}
            {@const idx = runDropdown.requestIdx}
            {@const req = parsedRequests[idx]}
            <button class="fixed inset-0 z-40 cursor-default" onclick={closeRunDropdown} aria-label="Close dropdown"></button>
            <ul
                class="fixed z-50
                    menu menu-sm bg-base-200
                    rounded-box shadow-sm border border-base-content/10
                    w-80 p-1"
                style="left: {runDropdown.x}px; top: {runDropdown.y}px;"
                role="menu"
            >
                <li>
                    <button onclick={() => sendRunRequest(idx)}>
                        <span class="truncate">Send {req?.title || `${req?.method ?? ''} ${req?.url ?? ''}`.trim() || 'Request'}</span>
                        <span class="text-xs opacity-50 ml-auto">{app.modKey}+Enter</span>
                    </button>
                </li>
            </ul>
        {/if}
    {:else if viewMode === "file"}
        <FileOverview
            requests={parsedRequests}
            variables={fileVariables}
            {activeRequestIdx}
            onNavigateToRequest={handleNavigateToRequest}
            onVariablesUpdate={handleVariablesUpdate}
            initialSubTab={fileOverviewTab}
            onSubTabChange={handleFileOverviewTabChange}
        />
    {:else}
        <div class="shrink-0 border-b border-base-300 overflow-hidden" style="height: {requestHeight}px">
            <RequestForm
                request={formRequest}
                loading={sending}
                onSend={handleSend}
                onUpdate={handleFormUpdate}
                defaultHeaders={app.defaultHeaders}
                initialTab={requestFormTab}
                onTabChange={handleRequestFormTabChange}
                onUrlBlur={handleUrlBlur}
            />
        </div>
        <div role="presentation"
            class="h-1 bg-base-300 hover:bg-primary cursor-row-resize shrink-0 transition-colors"
            onmousedown={onSplitterStart}
        ></div>
        <div class="flex-1 overflow-hidden">
            <ResponseView {result} loading={sending} error={reqError} />
        </div>
    {/if}
</div>
