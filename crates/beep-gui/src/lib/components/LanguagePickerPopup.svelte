<script lang="ts">
    // Zed-style language picker: type to filter, arrows to move, Enter / click to pick, Esc / click outside to close.
    import type { EditorLanguage } from "$lib/types";
    import { LANGUAGE_LABELS } from "$lib/language-sniff";

    interface Props {
        open: boolean;
        current: EditorLanguage | null;
        onSelect: (lang: EditorLanguage) => void;
        onClose: () => void;
    }

    let { open, current, onSelect, onClose }: Props = $props();

    const LANGUAGES = Object.keys(LANGUAGE_LABELS) as EditorLanguage[];

    let input: HTMLInputElement | undefined = $state();
    let text = $state("");
    let selectedIdx = $state(0);
    let listEl: HTMLElement | undefined = $state();

    const items = $derived(
        LANGUAGES.filter((l) => LANGUAGE_LABELS[l].toLowerCase().includes(text.trim().toLowerCase())),
    );

    $effect(() => {
        if (!open) return;
        text = "";
        selectedIdx = Math.max(0, current ? LANGUAGES.indexOf(current) : 0);
        queueMicrotask(() => input?.focus());
    });

    $effect(() => {
        listEl?.children[selectedIdx]?.scrollIntoView({ block: "nearest" });
    });

    function pick(lang: EditorLanguage | undefined) {
        if (lang) onSelect(lang);
        onClose();
    }

    function onkeydown(e: KeyboardEvent) {
        if (e.key === "ArrowDown") {
            e.preventDefault();
            selectedIdx = Math.min(selectedIdx + 1, items.length - 1);
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            selectedIdx = Math.max(selectedIdx - 1, 0);
        } else if (e.key === "Enter") {
            e.preventDefault();
            pick(items[selectedIdx]);
        } else if (e.key === "Escape") {
            e.preventDefault();
            onClose();
        }
    }
</script>

{#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <dialog class="modal items-start" open onclick={onClose}>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
            class="modal-box p-0 w-96 mt-[10vh] border border-base-content/10"
            onclick={(e: MouseEvent) => e.stopPropagation()}
        >
            <input
                bind:this={input}
                bind:value={text}
                oninput={() => (selectedIdx = 0)}
                {onkeydown}
                class="w-full bg-transparent px-3 py-2 text-sm outline-none border-b border-base-content/10"
                placeholder="Select a language..."
                spellcheck="false"
                aria-label="Select language"
            />
            <div bind:this={listEl} class="max-h-72 overflow-y-auto p-1" role="listbox">
                {#each items as lang, i (lang)}
                    <div
                        class="flex items-center justify-between px-2 py-1 text-sm rounded cursor-pointer {i === selectedIdx ? 'bg-primary/80' : 'hover:bg-base-300'}"
                        onclick={() => pick(lang)}
                        onmousemove={() => (selectedIdx = i)}
                        role="option"
                        aria-selected={i === selectedIdx}
                        tabindex="-1"
                    >
                        <span>{LANGUAGE_LABELS[lang]}</span>
                        {#if lang === current}
                            <span class="text-xs opacity-60">current</span>
                        {/if}
                    </div>
                {:else}
                    <div class="px-2 py-1 text-sm opacity-50">No matches</div>
                {/each}
            </div>
        </div>
    </dialog>
{/if}
