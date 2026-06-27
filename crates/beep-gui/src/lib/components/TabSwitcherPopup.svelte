<script lang="ts">
    import type { Tab } from "$lib/types";

    interface Props {
        open: boolean;
        tabs: Tab[];
        activeTabId: string;
        onSelect: (id: string) => void;
        onClose: () => void;
    }

    let {
        open,
        tabs,
        activeTabId,
        onSelect,
        onClose,
    }: Props = $props();

    let selectedIdx = $state(0);

    // Order by lastActiveAt descending (most recent first), current tab last
    let orderedTabs = $derived.by(() => {
        const others = tabs
            .filter((t) => t.id !== activeTabId)
            .sort((a, b) => (b.lastActiveAt ?? 0) - (a.lastActiveAt ?? 0));
        const current = tabs.find((t) => t.id === activeTabId);
        if (current) others.push(current);
        return others;
    });

    // Reset selection when popup opens. index 0 = previous active tab
    $effect(() => {
        if (open) {
            selectedIdx = 0;
        }
    });

    function confirmAndClose() {
        const tab = orderedTabs[selectedIdx];
        if (tab) onSelect(tab.id);
    }

    // Global keyboard handling for popup navigation + Ctrl-release-to-confirm.
    // Always-active so Ctrl keyup is caught even if popup just opened.
    $effect(() => {
        function onKeyDown(e: KeyboardEvent) {
            if (!open) return;

            if (e.key === "Tab") {
                e.preventDefault();
                if (e.shiftKey) {
                    selectedIdx = selectedIdx > 0 ? selectedIdx - 1 : orderedTabs.length - 1;
                } else {
                    selectedIdx = (selectedIdx + 1) % orderedTabs.length;
                }
            } else if (e.key === "ArrowDown") {
                e.preventDefault();
                selectedIdx = Math.min(selectedIdx + 1, orderedTabs.length - 1);
            } else if (e.key === "ArrowUp") {
                e.preventDefault();
                selectedIdx = Math.max(selectedIdx - 1, 0);
            } else if (e.key === "Enter") {
                e.preventDefault();
                confirmAndClose();
            } else if (e.key === "Escape") {
                e.preventDefault();
                onClose();
            }
        }

        function onKeyUp(e: KeyboardEvent) {
            if (!open) return;
            if (e.key === "Control") {
                confirmAndClose();
            }
        }

        window.addEventListener("keydown", onKeyDown, true);
        window.addEventListener("keyup", onKeyUp, true);

        return () => {
            window.removeEventListener("keydown", onKeyDown, true);
            window.removeEventListener("keyup", onKeyUp, true);
        };
    });
</script>

{#if open && orderedTabs.length > 0}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <dialog class="modal items-start" open onclose={onClose} onclick={onClose}>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
            class="modal-box p-0 w-96 mt-[10vh]"
            onclick={(e: MouseEvent) => e.stopPropagation()}
        >
            <div class="px-2 py-1 text-xs uppercase tracking-wider opacity-50 font-semibold border-b border-base-content/10">
                Tab
            </div>
            <div class="max-h-60 overflow-y-auto">
                {#each orderedTabs as tab, i}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div
                        class="flex items-center gap-2 px-2 py-1.5 cursor-pointer {i === selectedIdx ? 'bg-primary/80' : 'hover:bg-base-300'}"
                        class:italic={!tab.persistent}
                        onclick={() => onSelect(tab.id)}
                        role="button"
                        tabindex="0"
                    >
                        <span class="text-sm truncate">{tab.label}</span>
                    </div>
                {/each}
            </div>
        </div>
    </dialog>
{/if}
