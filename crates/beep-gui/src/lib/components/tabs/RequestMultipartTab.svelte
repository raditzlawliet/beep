<script lang="ts">
    import type { FormField } from "$lib/types";

    interface Props {
        initialValue: FormField[];
        onchange: (fields: FormField[]) => void;
    }

    let { initialValue = [], onchange }: Props = $props();

    type Row = { key: string; value: string; enabled: boolean; fieldType: "text" | "file" };
    let rows = $state<Row[]>([]);

    let _lastInit = $state("");

    function initFromProps() {
        const key = JSON.stringify(initialValue);
        if (key === _lastInit) return;
        _lastInit = key;
        rows = initialValue.map((f) => ({
            key: f.key,
            value: f.value,
            enabled: f.enabled,
            fieldType: (f.field_type === "file" ? "file" : "text") as "text" | "file",
        }));
        ensureTempRow();
    }

    function ensureTempRow() {
        const last = rows[rows.length - 1];
        if (rows.length === 0 || (last && (last.key.trim() || last.value.trim()))) {
            rows.push({ key: "", value: "", enabled: true, fieldType: "text" });
        }
    }

    function emit() {
        const out: FormField[] = [];
        for (const r of rows) {
            if (r.key.trim()) {
                out.push({
                    key: r.key.trim(),
                    value: r.value,
                    enabled: r.enabled,
                    field_type: r.fieldType,
                });
            }
        }
        onchange(out);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const r = rows[idx];
        rows[idx] = { ...r, [field]: val };
        ensureTempRow();
        emit();
    }

    function setFieldType(idx: number, ft: "text" | "file") {
        const r = rows[idx];
        rows[idx] = { ...r, fieldType: ft };
        emit();
    }

    function removeRow(idx: number) {
        rows.splice(idx, 1);
        ensureTempRow();
        emit();
    }

    function toggleRow(idx: number) {
        const r = rows[idx];
        rows[idx] = { ...r, enabled: !r.enabled };
        emit();
    }

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
            <th class="w-12 text-xs">Type</th>
            <th class="w-0"></th>
        </tr>
    </thead>
    <tbody>
        {#each rows as row, i}
            {@const isLast = i === rows.length - 1 && !row.key.trim() && !row.value.trim()}
            <tr class="group hover:bg-base-300 divide-x divide-base-content/10">
                <td>
                    <input type="checkbox" class="checkbox checkbox-xs"
                        checked={row.enabled} disabled={isLast} hidden={isLast}
                        onchange={() => toggleRow(i)} />
                </td>
                <td>
                    <input class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Key" value={row.key}
                        oninput={(e) => updateRow(i, "key", (e.target as HTMLInputElement).value)} />
                </td>
                <td>
                    {#if row.fieldType === "text"}
                        <input class="input input-ghost input-xs w-full font-mono p-0"
                            placeholder="Value" value={row.value}
                            oninput={(e) => updateRow(i, "value", (e.target as HTMLInputElement).value)} />
                    {:else}
                        <span class="text-xs text-base-content/50 px-1">File upload (coming soon)</span>
                    {/if}
                </td>
                <td>
                    <select class="select select-ghost select-xs font-normal"
                        value={row.fieldType}
                        onchange={(e) => setFieldType(i, (e.target as HTMLSelectElement).value as "text" | "file")}>
                        <option value="text">Text</option>
                        <option value="file">File</option>
                    </select>
                </td>
                <td>
                    {#if !isLast}
                        <button class="btn btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100"
                            onclick={() => removeRow(i)}>✕</button>
                    {/if}
                </td>
            </tr>
        {/each}
    </tbody>
</table>
