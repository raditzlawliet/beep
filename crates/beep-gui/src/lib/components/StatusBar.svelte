<script lang="ts">
    import type { HttpResponse } from "$lib/types";
    import { HistoryIcon, FolderTree } from "@lucide/svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import HorizontalDivider from "./uis/HorizontalDivider.svelte";
    import { app } from "$lib/app-state.svelte";

    interface Props {
        onToggleHistoryFocus: () => void;
        onToggleProjectFocus: () => void;
        activePanel: "history" | "project";
        sidebarOpen: boolean;
        hasProject: boolean;
        response: HttpResponse | null;
        loading: boolean;
        error: string | null;
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
    }: Props = $props();
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
        <span class="flex items-center gap-1.5">
            <StatusBadge status={response.status} />
            <span class="opacity-30">·</span>
            <span class="opacity-70">{response.elapsed_ms}ms</span>
        </span>
    {:else}
        <span class="text-xs opacity-50">Ready</span>
    {/if}
</div>
