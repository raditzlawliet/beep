<script lang="ts">
    // File mode of a `.beep` env/vars JSON file: key/value form; edits rewrite the file text.
    import VarInput from "$lib/components/VarInput.svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";
    import { parseVarsJson, serializeVarsJson, typedValue, VAR_KINDS, type VarKind, type VarsRow } from "$lib/vars/beep-files";

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

    function changeKind(idx: number, kind: VarKind) {
        const v = rows[idx].value.trim();
        // empty / plain text has no valid object form; start from `{}`
        updateRow(idx, kind === "object" && !v.startsWith("{") && !v.startsWith("[") ? { kind, value: "{}" } : { kind });
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
    <table class="table table-xs table-pin-rows table-fixed w-full">
        <thead>
            <tr>
                <th class="w-8"></th>
                <!-- table-fixed: w-8 columns stay exact, the 4 auto columns share the rest; Value spans 2 -> key : value : description = 1 : 2 : 1 -->
                <th class="text-xs">Key</th>
                <th class="text-xs" colspan="2">Value</th>
                <th class="text-xs">Description</th>
                <!-- action column: just fits the delete button (td p-0) -->
                <th class="w-8"></th>
            </tr>
        </thead>
        <tbody>
            {#each rows as row, i (i)}
                <tr class="group hover:bg-base-300 divide-x divide-base-content/10" class:opacity-50={!row.enabled}>
                    <td class="text-center p-0">
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
                    <td colspan="2">
                        {#if row.kind === "other"}
                            <span class="font-mono text-xs opacity-50 tooltip tooltip-top" data-tip="null or an object without &quot;value&quot;">
                                {row.value} (ignored by Beep)
                            </span>
                        {:else}
                            {@const invalid = typedValue(row) === null}
                            <div class="flex items-center gap-1">
                                <div class="flex-1 min-w-0" class:tooltip={invalid} class:tooltip-top={invalid}
                                    data-tip={invalid ? `Not a valid ${row.kind} - saved as string until valid` : undefined}>
                                    <VarInput class="input input-ghost input-xs w-full font-mono p-0 {invalid ? 'underline decoration-wavy decoration-error' : ''}"
                                        placeholder="Value" value={row.value}
                                        oninput={(v) => updateRow(i, { value: v })} />
                                </div>
                                <select class="select select-ghost select-xs w-18 shrink-0 ps-1 pe-5 opacity-60 hover:opacity-100"
                                    aria-label="Value type" value={row.kind}
                                    onchange={(e) => changeKind(i, (e.target as HTMLSelectElement).value as VarKind)}>
                                    {#each VAR_KINDS as k (k)}
                                        <option value={k}>{k}</option>
                                    {/each}
                                </select>
                            </div>
                        {/if}
                    </td>
                    <td>
                        {#if row.kind !== "other"}
                            <input class="input input-ghost input-xs w-full p-0"
                                placeholder="Description" value={row.description}
                                oninput={(e) => updateRow(i, { description: (e.target as HTMLInputElement).value })} />
                        {/if}
                    </td>
                    <td class="text-center p-0">
                        <DeleteRowButton label="Delete variable" onclick={() => removeRow(i)} />
                    </td>
                </tr>
            {/each}
            <tr>
                <td class="p-0" colspan="6">
                    <AddRowButton onclick={addRow} text="Add variable" />
                </td>
            </tr>
        </tbody>
    </table>
{/if}
