<script lang="ts">
    import type { HistoryEntry } from "$lib/types";
    import MethodBadge from "$lib/components/MethodBadge.svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import { BrushCleaning, Trash2 } from "@lucide/svelte";

    interface Props {
        entries: HistoryEntry[];
        onSelect: (entry: HistoryEntry) => void;
        onClearAll: () => void;
        onDelete: (id: number) => void;
    }

    let { entries, onSelect, onClearAll, onDelete }: Props = $props();

    let listEl = $state<HTMLUListElement>();

    // auto-scroll to top when new entries are added (latest on top)
    $effect(() => {
        // track entries length to trigger
        entries.length;
        if (listEl) {
            requestAnimationFrame(() => {
                if (listEl) {
                    listEl.scrollTop = 0;
                }
            });
        }
    });
</script>

<div class="flex flex-col h-full border-r border-base-300 w-full">
    <div
        class="py-2 text-xs font-semibold opacity-70 uppercase tracking-wide select-none border-b border-b-base-content/10 flex items-center justify-between h-10"
    >
        <span class="px-2">History</span>
        {#if entries.length > 0}
            <button
                class="btn btn-ghost btn-xs btn-square me-1"
                onclick={onClearAll}
                title="Clear all history"
            >
                <BrushCleaning class="h-3.5 w-3.5" />
            </button>
        {/if}
    </div>
    <ul
        bind:this={listEl}
        class="menu menu-sm flex-1 overflow-y-auto flex-nowrap rounded-none p-0 [&_li]:rounded-none w-full"
    >
        {#if entries.length === 0}
            <li class="flex-1">
                <span class="text-xs opacity-40 py-2">No requests yet</span>
            </li>
        {:else}
            {#each entries.toReversed() as entry (entry.id)}
                {console.log(entry.request)}
                <li class="group relative flex items-center">
                    <button
                        class="text-xs py-2 w-full"
                        onclick={() => onSelect(entry)}
                    >
                        <span class="flex items-center gap-1.5 min-w-0">
                            <MethodBadge method={entry.request.method} />
                            {#if entry.response}
                                <StatusBadge
                                    status={entry.response.status}
                                    showLabel={false}
                                />
                            {/if}
                            <span
                                class="truncate opacity-70"
                                title={entry.request.url || "(empty)"}
                                >{entry.request.url || "(empty)"}</span
                            >
                        </span>
                    </button>
                    <div
                        class="absolute right-1 top-1/2 -translate-y-1/2
                                hidden group-hover:flex items-center gap-0.5 bg-transparent p-0 m-0"
                    >
                        <button
                            class="btn btn-ghost btn-xs btn-square"
                            onclick={(e) => {
                                e.stopPropagation();
                                onDelete(entry.id);
                            }}
                            title="Remove"
                        >
                            <Trash2 class="h-4 w-4" />
                        </button>
                    </div>
                </li>
            {/each}
        {/if}
    </ul>
</div>
