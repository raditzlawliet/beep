<script lang="ts">
    import type {
        Tab,
        HttpRequest,
        ParsedRequest,
        ParsedFileVariable,
        ViewMode,
    } from "$lib/types";
    import { emptyParsedRequest } from "$lib/types";
    import { app, httpFile, project } from "$lib/app-state.svelte";
    import { parsedToHttpRequest, httpRequestToParsed } from "$lib/http-file-utils";
    import MainEditorToolbar from "$lib/components/MainEditorToolbar.svelte";
    import FileEditor from "$lib/components/FileEditor.svelte";
    import RequestForm from "$lib/components/RequestForm.svelte";
    import ResponseView from "$lib/components/ResponseView.svelte";
    import FileOverview from "$lib/components/FileOverview.svelte";

    interface Props {
        tab: Tab;
        sending: boolean;
        reqError: string | null;
        response: import("$lib/types").HttpResponse | null;
        onContentChange: (newContent: string) => void;
        onTabStateChange: (state: Partial<Tab>) => void;
        onSend: (req: HttpRequest) => void;
        requestHeight: number;
        onSplitterStart: (e: MouseEvent) => void;
    }

    let {
        tab,
        sending,
        reqError,
        response,
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
    let formRequest = $state<HttpRequest>(parsedToHttpRequest(undefined));
    let requestFormTab = $state<string>("params");
    let fileOverviewTab = $state<string>("requests");

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
        formRequest = parsedToHttpRequest(parsedRequests[activeRequestIdx]);
    });

    // Parse content when it changes
    $effect(() => {
        const content = tab.content;
        if (!content && parsedRequests.length === 0) {
            viewMode = "request";
            activeRequestIdx = 0;
            parsedRequests = [emptyParsedRequest()];
            fileVariables = [];
            formRequest = parsedToHttpRequest(parsedRequests[0]);
            saveTabState({ viewMode, activeRequestIdx: 0, parsedRequests, fileVariables });
            return;
        }
        if (!content) return;

        httpFile.parse(content).then((result) => {
            parsedRequests = result.requests;
            fileVariables = result.variables;
            if (parsedRequests.length === 0) {
                parsedRequests = [emptyParsedRequest()];
                activeRequestIdx = 0;
            } else if (activeRequestIdx >= parsedRequests.length) {
                activeRequestIdx = Math.max(0, parsedRequests.length - 1);
            }
            if (viewMode === "request") {
                formRequest = parsedToHttpRequest(parsedRequests[activeRequestIdx]);
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
            if (pos >= parsedRequests[i].offset_start && pos <= parsedRequests[i].offset_end + 1) {
                found = i; break;
            }
        }
        if (found === -1 && parsedRequests.length > 0) {
            found = pos < (parsedRequests[0]?.offset_start ?? 0) ? 0 : parsedRequests.length - 1;
        }
        if (found !== -1 && found !== activeRequestIdx) {
            activeRequestIdx = found;
            saveTabState({ activeRequestIdx: found });
            if (viewMode === "request") formRequest = parsedToHttpRequest(parsedRequests[found]);
        }
    }

    function handleSelectRequest(idx: number) {
        activeRequestIdx = idx;
        saveTabState({ activeRequestIdx: idx });
        if (viewMode === "request") formRequest = parsedToHttpRequest(parsedRequests[idx]);
    }

    function handleSetMode(mode: ViewMode) {
        if (viewMode === mode) return;
        // sync before leaving request mode
        if (viewMode === "request" && mode !== "request") {
            syncFormToContent();
        }
        viewMode = mode;
        if (mode === "request") {
            formRequest = parsedToHttpRequest(parsedRequests[activeRequestIdx]);
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
        formRequest = parsedToHttpRequest(parsedRequests[idx]);
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
        const updated = httpRequestToParsed(formRequest, parsedRequests[activeRequestIdx]);
        parsedRequests[activeRequestIdx] = updated;
        saveTabState({ parsedRequests });
        try {
            const newContent = await httpFile.updateRequest(tab.content, activeRequestIdx, updated);
            onContentChange(newContent);
        } catch (e) {
            console.error("Failed to sync form to content:", e);
        }
    }

    function handleFormUpdate(req: HttpRequest) {
        formRequest = req;
        const updated = httpRequestToParsed(req, parsedRequests[activeRequestIdx]);
        parsedRequests[activeRequestIdx] = updated;
        saveTabState({ parsedRequests });
    }

    function handleSend(req: HttpRequest) {
        syncFormToContent();
        onSend(req);
    }

    let fileName = $derived(
        tab.filePath && project.path && tab.filePath.startsWith(project.path)
            ? tab.filePath.slice(project.path.length).replace(/^[/\\]/, "")
            : tab.label
    );
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
                language="text"
                wrapLines={true}
                onchange={handleCodeChange}
                initialCursorPos={cursorPos}
                oncursorchange={handleCursorChange}
                class="h-full"
            />
        </div>
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
            />
        </div>
        <div role="presentation"
            class="h-1 bg-base-300 hover:bg-primary cursor-row-resize shrink-0 transition-colors"
            onmousedown={onSplitterStart}
        ></div>
        <div class="flex-1 overflow-hidden">
            <ResponseView {response} loading={sending} error={reqError} />
        </div>
    {/if}
</div>
