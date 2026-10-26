<script lang="ts">
    import VarInput from "$lib/components/VarInput.svelte";
    import type { ParsedFileVariable } from "$lib/types";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";

    interface Props {
        initialValue: ParsedFileVariable[];
        onchange: (vars: ParsedFileVariable[]) => void;
    }

    let { initialValue = [], onchange }: Props = $props();

    let rows = $state<ParsedFileVariable[]>([]);

    // Track last synced value to detect external changes (history load, code edit).
    let _lastInit = $state("");

    function initFromProps() {
        const key = JSON.stringify(initialValue);
        if (key === _lastInit) return;
        _lastInit = key;
        rows = initialValue.map((v) => ({ key: v.key, value: v.value }));
    }

    function emit() {
        const out = rows.map((r) => ({ key: r.key.trim(), value: r.value }));
        _lastInit = JSON.stringify(out);
        onchange(out);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        rows[idx] = { ...rows[idx], [field]: val };
        emit();
    }

    function removeRow(idx: number) {
        rows.splice(idx, 1);
        emit();
    }

    function addRow() {
        rows = [...rows, { key: "", value: "" }];
        emit();
    }

    // Sync on mount and when props change externally.
    $effect(() => {
        void JSON.stringify(initialValue);
        initFromProps();
    });
</script>

<div class="px-2 py-1 text-[11px] opacity-50">
    Request <code>@var</code> - overrides file vars, used as <code>{"{{name}}"}</code>.
</div>
<table class="table table-xs table-pin-rows table-pin-cols min-w-max">
    <thead>
        <tr>
            <th class="w-auto text-xs">Key</th>
            <th class="w-auto text-xs">Value</th>
            <th class="w-0"></th>
        </tr>
    </thead>
    <tbody>
        {#each rows as row, i}
            <tr class="group hover:bg-base-300 divide-x divide-base-content/10">
                <td>
                    <input class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Key" value={row.key}
                        oninput={(e) => updateRow(i, "key", (e.target as HTMLInputElement).value)} />
                </td>
                <td>
                    <VarInput class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Value" value={row.value}
                        oninput={(v) => updateRow(i, "value", v)} />
                </td>
                <td>
                    <DeleteRowButton label="Delete variable" onclick={() => removeRow(i)} />
                </td>
            </tr>
        {/each}
        <tr>
            <td class="p-0">
                <AddRowButton onclick={addRow} text="Add variable" />
            </td>
        </tr>
    </tbody>
</table>
