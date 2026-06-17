<script lang="ts">
    import type { FormField } from "$lib/types";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";

    interface Props {
        initialValue: FormField[];
        onchange: (fields: FormField[]) => void;
    }

    let { initialValue = [], onchange }: Props = $props();

    type Row = { key: string; value: string; enabled: boolean };
    let rows = $state<Row[]>([]);

    // Track last synced value to detect external changes (history load).
    let _lastInit = $state("");

    function initFromProps() {
        const key = JSON.stringify(initialValue);
        if (key === _lastInit) return;
        _lastInit = key;
        rows = initialValue.map((f) => ({
            key: f.key,
            value: f.value,
            enabled: f.enabled,
        }));
    }

    function emit() {
        const out: FormField[] = [];
        for (const r of rows) {
            out.push({ key: r.key.trim(), value: r.value, enabled: r.enabled, field_type: "text", content_type: "" });
        }
        onchange(out);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const r = rows[idx];
        rows[idx] = { ...r, [field]: val };
        emit();
    }

    function removeRow(idx: number) {
        rows.splice(idx, 1);
        emit();
    }

    function addRow() {
        rows = [...rows, { key: "", value: "", enabled: true }];
        emit();
    }

    function toggleRow(idx: number) {
        const r = rows[idx];
        rows[idx] = { ...r, enabled: !r.enabled };
        emit();
    }

    // Sync on mount and when props change externally.
    $effect(() => {
        void JSON.stringify(initialValue);
        initFromProps();
    });
</script>

<table class="table table-xs table-pin-rows table-pin-cols min-w-max">
    <thead>
        <tr>
            <th class="w-0"><input type="checkbox" class="checkbox checkbox-xs invisible" /></th>
            <th class="w-auto text-xs">Key</th>
            <th class="w-auto text-xs">Value</th>
            <th class="w-0"></th>
        </tr>
    </thead>
    <tbody>
        {#each rows as row, i}
            <tr class="group hover:bg-base-300 divide-x divide-base-content/10">
                <td>
                    <input type="checkbox" class="checkbox checkbox-xs"
                        checked={row.enabled}
                        onchange={() => toggleRow(i)} />
                </td>
                <td>
                    <input class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Key" value={row.key}
                        oninput={(e) => updateRow(i, "key", (e.target as HTMLInputElement).value)} />
                </td>
                <td>
                    <input class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Value" value={row.value}
                        oninput={(e) => updateRow(i, "value", (e.target as HTMLInputElement).value)} />
                </td>
                <td>
                    <DeleteRowButton onclick={() => removeRow(i)} />
                </td>
            </tr>
        {/each}
        <tr>
            <td></td>
            <td class="p-0">
                <AddRowButton onclick={addRow} text="Add field" />
            </td>
        </tr>
    </tbody>
</table>
