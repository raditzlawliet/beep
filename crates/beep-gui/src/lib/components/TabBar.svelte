<script lang="ts">
    import type { Tab } from "$lib/types";
    import { XIcon } from "@lucide/svelte";

    interface Props {
        tabs: Tab[];
        activeTabId: string;
        onSelectTab: (id: string) => void;
        onCloseTab: (id: string) => void;
    }

    let { tabs, activeTabId, onSelectTab, onCloseTab }: Props = $props();

    function wheelHandler(e: WheelEvent) {
        const el = e.currentTarget as HTMLElement;
        el.scrollLeft += e.deltaY / 5;
    }

    function dotClass(tab: Tab): string {
        if (tab.type !== "file" || tab.originalContent === undefined) return "";
        if (tab.content === tab.originalContent) return "";
        if (tab.diskChanged) return "bg-warning";
        return "bg-primary";
    }
</script>

<div
    class="flex items-center bg-base-200/50 overflow-x-auto shrink-0 tab-bar-scroll"
    onwheel={wheelHandler}
>
    {#each tabs as tab (tab.id)}
        <div
            role="tab"
            tabindex="0"
            class="group flex items-center
                gap-1.5 h-8 px-2 text-xs
                {activeTabId === tab.id ? 'text-base-content' : 'text-base-content/50'}
                border-r border-base-content/10
                hover:bg-base-200 transition-colors
                shrink-0 max-w-48 cursor-pointer"
            class:bg-base-100={activeTabId === tab.id}
            class:border-b={activeTabId !== tab.id}
            class:border-t-2={activeTabId === tab.id}
            class:border-t-primary={activeTabId === tab.id}
            class:italic={!tab.persistent}
            onclick={() => onSelectTab(tab.id)}
            onkeydown={(e: KeyboardEvent) => {
                if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onSelectTab(tab.id);
                }
            }}
        >
            {#if dotClass(tab)}
                <span class="w-1.5 h-1.5 rounded-full shrink-0 {dotClass(tab)}" aria-label="Unsaved changes"></span>
            {:else}
                <span class="w-1.5 h-1.5 shrink-0"></span>
            {/if}
            <span class="truncate">{tab.label}</span>
            {#if tab.type !== "request"}
                <button
                    class="btn btn-ghost btn-xs p-0 shrink-0 m-0
                        opacity-0 group-hover:opacity-50 hover:opacity-100!
                        rounded hover:bg-base-300"
                    onclick={(e: MouseEvent) => {
                        e.stopPropagation();
                        onCloseTab(tab.id);
                    }}
                    aria-label="Close tab"
                >
                    <XIcon class="h-3 w-3" />
                </button>
            {:else}
                <div class="p-1"></div>
            {/if}
        </div>
    {/each}
    <div class="flex-1 h-8 border-b border-base-content/10"></div>
</div>

<style>
    .tab-bar-scroll {
        scrollbar-width: none;
    }
    .tab-bar-scroll::-webkit-scrollbar {
        display: none;
    }
</style>
