<script lang="ts">
    interface Props {
        open: boolean;
        activeFileName: string | null;
        onNewUntitled: () => void;
        onNewInFile: () => void;
        onClose: () => void;
    }

    let {
        open,
        activeFileName,
        onNewUntitled,
        onNewInFile,
        onClose,
    }: Props = $props();

    let selectedIdx = $state(0);

    $effect(() => {
        if (open) selectedIdx = 0;
    });

    function buildItems(): { label: string; desc: string; action: () => void }[] {
        const list: { label: string; desc: string; action: () => void }[] = [
            {
                label: "New Request with New File",
                desc: "Create untitled .http file",
                action: onNewUntitled,
            },
        ];
        if (activeFileName) {
            list.push({
                label: `New Request in "${activeFileName}"`,
                desc: "Add request to current file",
                action: onNewInFile,
            });
        }
        return list;
    }

    let items = $derived(buildItems());

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "ArrowDown") {
            e.preventDefault();
            selectedIdx = Math.min(selectedIdx + 1, items.length - 1);
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            selectedIdx = Math.max(selectedIdx - 1, 0);
        } else if (e.key === "Enter") {
            e.preventDefault();
            items[selectedIdx]?.action();
        } else if (e.key === "Escape") {
            onClose();
        }
    }
</script>

{#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <dialog class="modal items-start" open onclose={onClose} onclick={onClose}>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
            class="modal-box p-0 w-72 mt-[10vh]"
            onclick={(e: MouseEvent) => e.stopPropagation()}
            onkeydown={(e: KeyboardEvent) => e.stopPropagation()}
        >
            <div class="px-2 py-1 text-xs uppercase tracking-wider opacity-50 font-semibold border-b border-base-content/10">
                New
            </div>
            <div class="">
                {#each items as item, i}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div
                        class="flex flex-col px-2 py-1.5 cursor-pointer {i === selectedIdx ? 'bg-primary/80' : 'hover:bg-base-300'}"
                        onclick={() => item.action()}
                        role="button"
                        tabindex="0"
                    >
                        <span class="text-sm">{item.label}</span>
                        <span class="opacity-50 text-xs">{item.desc}</span>
                    </div>
                {/each}
            </div>
        </div>
    </dialog>
{/if}

<svelte:window onkeydown={open ? handleKeydown : undefined} />
