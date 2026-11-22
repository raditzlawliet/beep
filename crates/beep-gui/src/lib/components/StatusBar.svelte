<script lang="ts">
    import type { EditorLanguage, HttpResponse } from "$lib/types";
    import { LANGUAGE_LABELS } from "$lib/language-sniff";
    import { HistoryIcon, FolderTree } from "@lucide/svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import { app } from "$lib/app-state.svelte";
    import LanguagePickerPopup from "$lib/components/LanguagePickerPopup.svelte";
    import { varIndex } from "$lib/vars/var-index.svelte";
    import { TriangleAlertIcon } from "@lucide/svelte";
    import { editorCursor } from "$lib/editor-cursor.svelte";

    interface Props {
        onToggleHistoryFocus: () => void;
        onToggleProjectFocus: () => void;
        activePanel: "history" | "project";
        sidebarOpen: boolean;
        hasProject: boolean;
        response: HttpResponse | null;
        loading: boolean;
        error: string | null;
        /** Active file tab language; null hides the picker. */
        language?: EditorLanguage | null;
        onLanguageSelect?: (lang: EditorLanguage) => void;
    }

    let {
        onToggleHistoryFocus,
        onToggleProjectFocus,
        activePanel,
        sidebarOpen,
        hasProject,
        response,
        loading,
        error,
        language = null,
        onLanguageSelect,
    }: Props = $props();

    let languageOpen = $state(false);
</script>

<div
    class="flex items-center h-8 mx-1 border-t border-t-base-content/10 gap-1 text-xs"
>
    <div class="tooltip tooltip-start">
        <button
            class="btn btn-ghost btn-xs btn-square"
            onclick={onToggleHistoryFocus}
            aria-label="Toggle History Panel"
        >
            <span class:text-primary={sidebarOpen && activePanel === "history"}>
                <HistoryIcon class="h-3 w-3" />
            </span>
        </button>
        <div class="tooltip-content text-xs">
            <span>History Panel</span>
            <span class="opacity-50 ml-4">{app.modKey}+Shift+H</span>
        </div>
    </div>
    <div class="tooltip tooltip-start">
        <button
            class="btn btn-ghost btn-xs btn-square"
            onclick={onToggleProjectFocus}
            aria-label="Toggle Project Panel"
        >
            <span class:text-primary={sidebarOpen && activePanel === "project"}>
                <FolderTree class="h-3 w-3" />
            </span>
        </button>
        <div class="tooltip-content text-xs">
            <span>Project Panel</span>
            <span class="opacity-50 ml-4">{app.modKey}+Shift+E</span>
        </div>
    </div>
    <div class="divider divider-horizontal w-1 m-0"></div>

    {#if loading}
        <span class="text-xs opacity-70">Loading...</span>
    {:else if error}
        <span class="text-xs text-error">{error}</span>
    {:else if response}
        <span class="flex items-center gap-1.5 font-mono">
            <StatusBadge status={response.status} />
            <span class="opacity-30">·</span>
            <span class="opacity-70">{response.elapsed_ms}ms</span>
        </span>
    {:else}
        <span class="text-xs opacity-50">Ready</span>
    {/if}
    <div class="flex-1"></div>
    {#if varIndex.unknown.length > 0}
        {@const unknown = varIndex.unknown}
        <div class="tooltip tooltip-top">
            <span class="flex items-center gap-1 text-warning px-1">
                <TriangleAlertIcon class="h-3 w-3" />
                {unknown.length} unknown {unknown.length === 1 ? "var" : "vars"}
            </span>
            <div class="tooltip-content text-xs text-left">
                <div class="opacity-70 mb-1">Sent as text:</div>
                {#each unknown as key (key)}
                    <div class="font-mono">{`{{${key}}}`}</div>
                {/each}
            </div>
        </div>
    {/if}
    {#if editorCursor.current}
        {@const cur = editorCursor.current}
        <div class="tooltip tooltip-top tooltip-end">
            <button
                class="btn btn-ghost btn-xs font-normal font-mono opacity-70 hover:opacity-100"
                onclick={() => editorCursor.openGoto()}
                aria-label="Go to Line/Column"
            >
                {cur.line}:{cur.col}
            </button>
            <div class="tooltip-content text-xs">
                <span>Go to Line/Column</span>
                <span class="opacity-50 ml-4">{app.modKey}+G</span>
            </div>
        </div>
    {/if}
    {#if language}
        <div class="tooltip tooltip-top tooltip-end">
            <button
                class="btn btn-ghost btn-xs font-normal opacity-70 hover:opacity-100"
                aria-label="Select Language"
                onclick={() => (languageOpen = true)}
            >
                {LANGUAGE_LABELS[language]}
            </button>
            <div class="tooltip-content text-xs">Select Language</div>
        </div>
    {/if}
</div>

<LanguagePickerPopup
    open={languageOpen}
    current={language}
    onSelect={(lang) => onLanguageSelect?.(lang)}
    onClose={() => (languageOpen = false)}
/>
