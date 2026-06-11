<script lang="ts">
    import type { HttpResponse } from "$lib/types";
    import { PanelLeft, HistoryIcon } from "@lucide/svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";

    interface Props {
        onToggleSidebar: () => void;
        response: HttpResponse | null;
        loading: boolean;
        error: string | null;
    }

    let { onToggleSidebar, response, loading, error }: Props = $props();
</script>

<div
    class="flex items-center h-8 mx-1 border-t border-t-base-content/10 gap-1 text-xs"
>
    <button
        class="btn btn-ghost btn-xs btn-square"
        onclick={onToggleSidebar}
        aria-label="Toggle sidebar"
    >
        <HistoryIcon class="h-4 w-4" />
    </button>
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
