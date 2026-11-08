<script lang="ts">
    // Zed-style "New environment" name input: Enter creates, Esc / click outside cancels.
    import { environment } from "$lib/app-state.svelte";

    interface Props {
        open: boolean;
        onclose: () => void;
        /** Called with the new env file path after create + select. */
        oncreated: (path: string) => void;
    }

    let { open, onclose, oncreated }: Props = $props();

    let input: HTMLInputElement | undefined = $state();
    let name = $state("");
    let error = $state<string | null>(null);

    $effect(() => {
        if (!open) return;
        name = "";
        error = null;
        queueMicrotask(() => input?.focus());
    });

    async function confirm() {
        if (!name.trim()) return;
        try {
            const path = await environment.create(name);
            onclose();
            oncreated(path);
        } catch (e) {
            error = String(e);
        }
    }

    function onkeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            confirm();
        } else if (e.key === "Escape") {
            e.preventDefault();
            onclose();
        }
    }
</script>

{#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <dialog class="modal items-start bg-transparent!" open onclick={onclose}>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
            class="modal-box p-0 w-96 mt-[10vh] border border-base-content/10"
            onclick={(e: MouseEvent) => e.stopPropagation()}
        >
            <input
                bind:this={input}
                bind:value={name}
                oninput={() => (error = null)}
                {onkeydown}
                class="w-full bg-transparent px-3 py-2 text-sm font-mono outline-none"
                placeholder="New environment name"
                spellcheck="false"
                aria-label="New environment name"
            />
            <div class="px-3 py-1 text-xs border-t border-base-content/10" class:text-error={error} class:opacity-50={!error}>
                {error ?? (name.trim() ? `Creates .beep/env.${name.trim()}.json` : "Enter to create, Esc to cancel")}
            </div>
        </div>
    </dialog>
{/if}
