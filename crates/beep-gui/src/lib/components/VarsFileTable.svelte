<script lang="ts">
    // File mode of a `.beep` env/vars JSON file: key/value form; edits rewrite the file text.
    import VarInput from "$lib/components/VarInput.svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";
    import { parseVarsJson, serializeVarsJson, type VarsRow } from "$lib/vars/beep-files";

    interface Props {
        content: string;
        onchange: (content: string) => void;
        onShowCode: () => void;
    }

    let { content, onchange, onShowCode }: Props = $props();

    let rows = $state<VarsRow[]>([]);
    let error = $state<string | null>(null);
    // Last text we parsed or wrote - skip re-parsing our own edits.
    let _last: string | null = null;

    $effect(() => {
        if (content === _last) return;
        _last = content;
        const parsed = parseVarsJson(content);
        if (typeof parsed === "string") {
            error = parsed;
        } else {
            error = null;
            rows = parsed;
        }
    });

    function emit() {
        const eol = content.includes("\r\n") ? "\r\n" : "\n";
        const text = serializeVarsJson(rows, eol);
        _last = text;
        onchange(text);
    }

    function updateRow(idx: number, patch: Partial<VarsRow>) {
        rows[idx] = { ...rows[idx], ...patch };
        emit();
    }

    function removeRow(idx: number) {
        rows.splice(idx, 1);
        emit();
    }

    function addRow() {
        rows = [...rows, { key: "", value: "", enabled: true, description: "", kind: "string" }];
    }
</script>

{#if error}
    <div class="p-4 text-sm flex flex-col items-start gap-2">
        <span class="text-error">Invalid JSON: {error}</span>
        <button class="btn btn-xs" onclick={onShowCode}>Open code view</button>
    </div>
{:else}
    <table class="table table-xs table-pin-rows min-w-max">
        <thead>
            <tr>
                <th class="w-0"></th>
                <th class="w-auto text-xs">Key</th>
                <th class="w-auto text-xs">Value</th>
                <th class="w-auto text-xs">Description</th>
                <th class="w-0"></th>
            </tr>
        </thead>
        <tbody>
            {#each rows as row, i (i)}
                <tr class="group hover:bg-base-300 divide-x divide-base-content/10" class:opacity-50={!row.enabled}>
                    <td>
                        <input type="checkbox" class="checkbox checkbox-xs" checked={row.enabled}
                            disabled={row.kind === "other"}
                            aria-label="Enabled"
                            onchange={(e) => updateRow(i, { enabled: (e.target as HTMLInputElement).checked })} />
                    </td>
                    <td>
                        <input class="input input-ghost input-xs w-full font-mono p-0"
                            placeholder="Key" value={row.key}
                            oninput={(e) => updateRow(i, { key: (e.target as HTMLInputElement).value })} />
                    </td>
                    <td>
                        {#if row.kind === "other"}
                            <span class="font-mono text-xs opacity-50 tooltip tooltip-top" data-tip="Needs a string, number or bool (or an object with &quot;value&quot;)">
                                {row.value} (ignored by Beep)
                            </span>
                        {:else}
                            <VarInput class="input input-ghost input-xs w-full font-mono p-0"
                                placeholder="Value" value={row.value}
                                oninput={(v) => updateRow(i, { value: v })} />
                        {/if}
                    </td>
                    <td>
                        {#if row.kind !== "other"}
                            <input class="input input-ghost input-xs w-full p-0"
                                placeholder="Description" value={row.description}
                                oninput={(e) => updateRow(i, { description: (e.target as HTMLInputElement).value })} />
                        {/if}
                    </td>
                    <td>
                        <DeleteRowButton label="Delete variable" onclick={() => removeRow(i)} />
                    </td>
                </tr>
            {/each}
            <tr>
                <td class="p-0" colspan="5">
                    <AddRowButton onclick={addRow} text="Add variable" />
                </td>
            </tr>
        </tbody>
    </table>
{/if}
