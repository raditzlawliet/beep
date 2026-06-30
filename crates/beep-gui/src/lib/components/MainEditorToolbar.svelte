<script lang="ts">
    import type { ParsedRequest, ViewMode, HttpMethod, TabType } from "$lib/types";
    import MethodBadge from "$lib/components/MethodBadge.svelte";
    import { ChevronDownIcon, CodeIcon, FileText, LayoutList } from "@lucide/svelte";
    import { app } from "$lib/app-state.svelte";

    interface Props {
        fileName: string;
        tabType: TabType;
        requests: ParsedRequest[];
        activeRequestIdx: number;
        viewMode: ViewMode;
        onSelectRequest: (idx: number) => void;
        onSetMode: (mode: ViewMode) => void;
        onSend: () => void;
    }

    let {
        fileName,
        tabType,
        requests,
        activeRequestIdx,
        viewMode,
        onSelectRequest,
        onSetMode,
        onSend,
    }: Props = $props();

    let requestDropdownOpen = $state(false);

    function toggleRequestDropdown(e: MouseEvent) {
        e.stopPropagation();
        requestDropdownOpen = !requestDropdownOpen;
    }

    function selectRequest(idx: number) {
        onSelectRequest(idx);
        requestDropdownOpen = false;
    }

    function handleDropdownKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            requestDropdownOpen = false;
        }
    }

    // Close dropdown on click outside
    function handleOutsideClick() {
        requestDropdownOpen = false;
    }
</script>

<svelte:window onclick={handleOutsideClick} />

<div class="flex items-center h-8 border-b border-base-content/10 py-0 text-xs shrink-0 select-none">
    <!-- File name -->
    <span class="opacity-60 font-mono shrink-0 ps-2">{fileName}</span>

    <!-- Request dropdown -->
    {#if requests.length > 0}
        <span class="opacity-60 font-mono ps-1">›</span>
        <div class="dropdown {requestDropdownOpen ? 'dropdown-open' : ''} flex-1 min-w-0">
            <button
                class="flex items-center gap-1 hover:bg-base-200 px-1 py-0.5 rounded text-xs font-mono w-full min-w-0"
                onclick={(e) => toggleRequestDropdown(e)}
                onkeydown={(e: KeyboardEvent) => {
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        e.stopPropagation();
                        requestDropdownOpen = !requestDropdownOpen;
                    }
                }}
            >
                <span class="shrink-0"><MethodBadge method={requests[activeRequestIdx]?.method as HttpMethod} /></span>
                <span class="truncate flex-1 min-w-0 text-left"
                    class:italic={!requests[activeRequestIdx]?.title}
                    class:opacity-80={!requests[activeRequestIdx]?.title}
                >{requests[activeRequestIdx]?.title || "Untitled Request"}</span>
                <ChevronDownIcon class="w-3 h-3 shrink-0 opacity-60 transform transition-transform duration-200 {requestDropdownOpen ? 'rotate-180' : ''}" />
            </button>
            {#if requestDropdownOpen}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="dropdown-content bg-base-200
                        rounded-box z-50 shadow-sm border border-base-content/10
                        w-full max-h-60 overflow-y-auto overflow-x-hidden p-1 mt-0.5"
                    onkeydown={handleDropdownKeydown}
                >
                    {#each requests as req, i}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div
                            class="flex items-center rounded cursor-pointer
                                px-2 py-1 text-xs gap-1
                                hover:bg-base-300 {i === activeRequestIdx ? 'bg-base-300' : ''}"
                            onclick={() => selectRequest(i)}
                            onkeydown={(e: KeyboardEvent) => {
                                if (e.key === "Enter" || e.key === " ") {
                                    e.preventDefault();
                                    selectRequest(i);
                                }
                            }}
                            role="option"
                            tabindex="0"
                            aria-selected={i === activeRequestIdx}
                        >
                            <MethodBadge method={req.method as HttpMethod} />
                            <span class="truncate"
                                class:italic={!req.title}
                                class:opacity-80={!req.title}
                            >{req.title || "Untitled Request"}</span>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {:else}
        <!-- spacer -->
        <div class="flex-1"></div>
    {/if}

    <!-- Mode tabs -->
    <div class="divider divider-horizontal w-1 m-0"></div>
    <div class="tabs tabs-bordered tabs-xs">
        <button
            class="tab gap-1 {viewMode === 'code' ? 'tab-active text-primary' : ''} tooltip tooltip-end"
            onclick={() => onSetMode('code')}
            title={`Code View    ${app.modKey}+1`}
        >
            <CodeIcon class="h-3.5 w-3.5" />
            <div class="tooltip-content text-xs">
                <span>Code View</span>
                <span class="opacity-50 ml-4">{app.modKey}+1</span>
            </div>
        </button>
        {#if tabType === 'http-file'}
            <button
                class="tab gap-1 {viewMode === 'file' ? 'tab-active text-primary' : ''} tooltip tooltip-end"
                onclick={() => onSetMode('file')}
                title={`File View    ${app.modKey}+2`}
            >
                <LayoutList class="h-3.5 w-3.5" />
                <div class="tooltip-content text-xs">
                    <span>File View</span>
                    <span class="opacity-50 ml-4">{app.modKey}+2</span>
                </div>
            </button>
            <button
                class="tab gap-1 {viewMode === 'request' ? 'tab-active text-primary' : ''} tooltip tooltip-end"
                onclick={() => onSetMode('request')}
                title={`Request View    ${app.modKey}+3`}
            >
                <FileText class="h-3.5 w-3.5" />
                <div class="tooltip-content text-xs">
                    <span>Request View</span>
                    <span class="opacity-50 ml-4">{app.modKey}+3</span>
                </div>
            </button>
        {/if}
    </div>
</div>
