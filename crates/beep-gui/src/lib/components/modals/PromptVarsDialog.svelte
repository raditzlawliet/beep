<script lang="ts">
    import { EyeIcon, EyeOffIcon } from "@lucide/svelte";
    import { SvelteSet } from "svelte/reactivity";
    import { promptVars } from "$lib/prompt-vars.svelte";

    let dialog = $state<HTMLDialogElement | null>(null);
    let values = $state<Record<string, string>>({});
    // Secret fields the user chose to show. A Set, so names like `constructor` or `__proto__` cannot hit inherited props.
    const shown = new SvelteSet<string>();

    function toggleShown(name: string) {
        if (!shown.delete(name)) shown.add(name);
    }

    const pending = $derived(promptVars.pending);

    $effect(() => {
        if (!dialog) return;
        if (pending) {
            values = Object.fromEntries(pending.prompts.map((p) => [p.name, promptVars.initial(p)]));
            shown.clear();
            dialog.showModal();
            // showModal focuses the first input; select so a prefilled value is easy to replace.
            dialog.querySelector<HTMLInputElement>("input")?.select();
        } else if (dialog.open) {
            dialog.close();
        }
    });

    function onsubmit(e: SubmitEvent) {
        e.preventDefault();
        promptVars.submit($state.snapshot(values));
    }
</script>

<!-- Esc fires `cancel`, which closes the dialog: treat it as Cancel. -->
<dialog class="modal" bind:this={dialog} oncancel={() => promptVars.cancel()}>
    <form class="modal-box border border-base-content/10 max-w-md" {onsubmit}>
        <h3 class="font-semibold mb-3">Request variables</h3>
        {#if pending}
            {#each pending.prompts as p (p.name)}
                {@const secret = p.secret}
                <div class="mb-3">
                    <label for="prompt-{p.name}" class="text-sm font-mono">{p.name}</label>
                    {#if p.description}
                        <span class="block text-xs opacity-60">{p.description}</span>
                    {/if}
                    <label class="input input-sm w-full mt-1">
                        <input
                            id="prompt-{p.name}"
                            class="grow"
                            type={secret && !shown.has(p.name) ? "password" : "text"}
                            autocomplete="off"
                            spellcheck="false"
                            bind:value={values[p.name]}
                        />
                        {#if secret}
                            <button
                                type="button"
                                class="opacity-60 hover:opacity-100"
                                tabindex="-1"
                                aria-label={shown.has(p.name) ? "Hide value" : "Show value"}
                                onclick={() => toggleShown(p.name)}
                            >
                                {#if shown.has(p.name)}<EyeOffIcon size={14} />{:else}<EyeIcon size={14} />{/if}
                            </button>
                        {/if}
                    </label>
                </div>
            {/each}
        {/if}
        <div class="modal-action mt-2">
            <button type="button" class="btn btn-sm btn-ghost" onclick={() => promptVars.cancel()}>Cancel</button>
            <button type="submit" class="btn btn-sm btn-primary">Send</button>
        </div>
    </form>
</dialog>
