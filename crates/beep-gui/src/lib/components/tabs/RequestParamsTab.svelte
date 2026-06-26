<script lang="ts">
    import type { QueryField } from "$lib/types";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";

    interface Props {
        initialValue: QueryField[];
        url: string;
        onchange: (params: QueryField[], url: string) => void;
    }

    let { initialValue = [], url = "", onchange }: Props = $props();

    type Row = { key: string; value: string; enabled: boolean; isInline: boolean };
    let rows = $state<Row[]>([]);

    // Sync from props, only when data actually differs.
    let _lastKey = $state("");
    $effect(() => {
        const key = JSON.stringify(initialValue);
        if (key !== _lastKey) {
            _lastKey = key;
            rows = initialValue.map((q) => ({
                key: q.key,
                value: q.value,
                enabled: q.enabled !== false,
                isInline: q.is_inline !== false,
            }));
            // Push display URL so the URL field reflects inline params
            emit();
        }
    });

    function emit() {
        const out: QueryField[] = rows.map((r) => ({
            key: r.key.trim(),
            value: r.value,
            enabled: r.enabled,
            is_inline: r.isInline,
        }));
        // Build display URL: base URL + inline enabled params
        const displayUrl = buildDisplayUrl(url, rows);
        onchange(out, displayUrl);
    }

    function buildDisplayUrl(baseUrl: string, r: Row[]): string {
        const inline = r.filter((row) => row.isInline && row.enabled && row.key.trim());
        if (inline.length === 0) return baseUrl;

        // Strip existing query from base
        const q = baseUrl.indexOf('?');
        const clean = q >= 0 ? baseUrl.slice(0, q) : baseUrl;
        const qs = inline.map((p) => `${encodeURIComponent(p.key.trim())}=${encodeURIComponent(p.value)}`).join("&");
        return `${clean}?${qs}`;
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        rows[idx] = { ...rows[idx], [field]: val };
        emit();
    }

    function toggleRow(idx: number) {
        const r = rows[idx];
        if (r.enabled) {
            // Disabling: inline to multiline. Never goes back.
            rows[idx] = { ...r, enabled: false, isInline: false };
        } else {
            // Enabling: stays at current isInline (never returns to true).
            rows[idx] = { ...r, enabled: true };
        }
        emit();
    }

    function removeRow(idx: number) {
        rows.splice(idx, 1);
        emit();
    }

    function addRow() {
        rows = [...rows, { key: "", value: "", enabled: true, isInline: true }];
        emit();
    }
</script>

<table class="table table-xs table-pin-rows table-pin-cols min-w-max">
    <thead>
        <tr>
            <th class="w-0"><input type="checkbox" class="checkbox checkbox-xs invisible" /></th>
            <th class="w-auto text-xs">Key</th>
            <th class="w-auto text-xs">Value</th>
            <th class="w-0 text-xs text-center">Source</th>
            <th class="w-0"></th>
        </tr>
    </thead>
    <tbody>
        {#each rows as row, i}
            <tr class="group hover:bg-base-300 divide-x divide-base-content/10">
                <td>
                    <input type="checkbox" class="checkbox checkbox-xs"
                        checked={row.enabled} onchange={() => toggleRow(i)} />
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
                <td class="text-center">
                    {#if row.isInline}
                        <span class="text-[10px] opacity-50" title="Inline in URL">URL</span>
                    {:else}
                        <span class="text-[10px] opacity-50" title="Multiline below URL">ML</span>
                    {/if}
                </td>
                <td>
                    <DeleteRowButton onclick={() => removeRow(i)} />
                </td>
            </tr>
        {/each}
        <tr>
            <td></td>
            <td class="p-0">
                <AddRowButton onclick={addRow} text="Add param" />
            </td>
        </tr>
    </tbody>
</table>
