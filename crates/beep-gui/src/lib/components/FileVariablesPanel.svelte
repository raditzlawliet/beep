<script lang="ts">
    import type { ParsedFileVariable } from "$lib/types";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";

    interface Props {
        variables: ParsedFileVariable[];
        collapsed: boolean;
        onUpdate: (variables: ParsedFileVariable[]) => void;
        onToggle: () => void;
    }

    let {
        variables,
        collapsed,
        onUpdate,
        onToggle,
    }: Props = $props();

    function updateKey(idx: number, key: string) {
        const updated = [...variables];
        updated[idx] = { ...updated[idx], key };
        onUpdate(updated);
    }

    function updateValue(idx: number, value: string) {
        const updated = [...variables];
        updated[idx] = { ...updated[idx], value };
        onUpdate(updated);
    }

    function removeRow(idx: number) {
        const updated = variables.filter((_, i) => i !== idx);
        onUpdate(updated);
    }

    function addRow() {
        const updated = [...variables, { key: "", value: "" }];
        onUpdate(updated);
    }
</script>

<div class="border-b border-base-content/10 bg-base-200/50 shrink-0">
    <!-- Toggle header -->
    <button
        class="flex items-center w-full px-2 py-0.5 text-[11px] opacity-60 hover:opacity-100 hover:bg-base-200 transition-colors"
        onclick={onToggle}
    >
        <span class="mr-1">{collapsed ? '▶' : '▼'}</span>
        <span>File Variables</span>
        {#if variables.length > 0}
            <span class="ml-1 opacity-50">({variables.length})</span>
        {/if}
    </button>

    {#if !collapsed}
        <div class="px-2 pb-1">
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="flex flex-col gap-0.5">
                {#each variables as v, i (i)}
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div class="flex items-center gap-1">
                        <input
                            type="text"
                            class="input input-xs input-bordered font-mono w-36 text-xs"
                            placeholder="key"
                            value={v.key}
                            oninput={(e) => updateKey(i, (e.target as HTMLInputElement).value)}
                        />
                        <span class="opacity-40 text-xs">=</span>
                        <input
                            type="text"
                            class="input input-xs input-bordered flex-1 font-mono text-xs"
                            placeholder="value"
                            value={v.value}
                            oninput={(e) => updateValue(i, (e.target as HTMLInputElement).value)}
                        />
                        <DeleteRowButton onclick={() => removeRow(i)} />
                    </div>
                {/each}
                <div class="mt-0.5">
                    <AddRowButton text="Variable" onclick={addRow} />
                </div>
            </div>
        </div>
    {/if}
</div>
