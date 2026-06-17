<script lang="ts">
    import { EyeIcon, EyeOffIcon } from "@lucide/svelte";
    import type { HeaderField } from "$lib/types";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";

    interface Props {
        initialValue: HeaderField[];
        defaultHeaders: [string, string][];
        onchange: (headers: HeaderField[]) => void;
    }

    let { initialValue = [], defaultHeaders = [], onchange }: Props = $props();

    type Row = { key: string; value: string; enabled: boolean; auto: boolean };
    let rows = $state<Row[]>([]);
    let showAutoHeaders = $state(false);

    let _lastInit = $state("");

    const overriddenKeys = $derived(
        new Set(
            rows
                .filter((r) => !r.auto && r.enabled && r.key.trim())
                .map((r) => r.key.trim().toLowerCase()),
        ),
    );

    function isOverridden(row: Row): boolean {
        return row.auto && row.key.trim() !== "" && overriddenKeys.has(row.key.trim().toLowerCase());
    }

    function normalizeHeaderKey(key: string) {
        return key.trim().toLowerCase();
    }

    function initFromProps() {
        const key = JSON.stringify({ initialValue, defaultHeaders });
        if (key === _lastInit) return;
        _lastInit = key;

        rows = [];

        const defaultsByKey = new Map(
            defaultHeaders.map(([key, value]) => [normalizeHeaderKey(key), { key, value }]),
        );

        // All headers from initialValue (preserves enabled/auto from history).
        const allKeys = new Set(initialValue.map((h) => normalizeHeaderKey(h.key)));
        for (const h of initialValue) {
            // rows.push({ key: h.key, value: h.value, enabled: h.enabled, auto: h.auto });
            const defaultHeader = h.auto ? defaultsByKey.get(normalizeHeaderKey(h.key)) : undefined;
            if (h.auto && !defaultHeader) continue;
            rows.push({
                key: defaultHeader?.key ?? h.key,
                value: defaultHeader?.value ?? h.value,
                enabled: h.enabled,
                auto: h.auto,
            });
        }

        // Auto-generated headers; skip if already present in initialValue.
        for (const [k, v] of defaultHeaders) {
            if (!allKeys.has(normalizeHeaderKey(k))) {
                rows.push({ key: k, value: v, enabled: true, auto: true });
            }
        }
        emit();
    }

    $effect(() => {
        void JSON.stringify(initialValue);
        initFromProps();
    });

    function emit() {
        const out: HeaderField[] = rows.map((r) => ({
            key: r.key.trim(),
            value: r.value,
            enabled: r.enabled,
            auto: r.auto,
        }));
        _lastInit = JSON.stringify({ initialValue: out, defaultHeaders });
        onchange(out);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const r = rows[idx];
        if (r.auto) return;
        rows[idx] = { ...r, [field]: val };
        emit();
    }

    function removeRow(idx: number) {
        if (rows[idx].auto) return;
        rows.splice(idx, 1);
        emit();
    }

    function addRow() {
        rows = [...rows, { key: "", value: "", enabled: true, auto: false }];
        emit();
    }

    function toggleRow(idx: number) {
        const r = rows[idx];
        rows[idx] = { ...r, enabled: !r.enabled };
        emit();
    }

    const autoHeaderCount = $derived(
        rows.filter((r) => r.auto && r.enabled && r.key.trim()).length,
    );
</script>

<div class="flex items-center justify-between px-1 pb-1">
    <button
        class="btn btn-ghost btn-xs gap-1 text-xs font-normal opacity-60 hover:opacity-100"
        onclick={() => (showAutoHeaders = !showAutoHeaders)}
    >
        {#if showAutoHeaders}
            <EyeOffIcon class="w-2.5 h-2.5" />
            <span>Hide auto-generated headers</span>
        {:else}
            <EyeIcon class="w-2.5 h-2.5" />
            <span>{autoHeaderCount} Hidden</span>
        {/if}
    </button>
</div>

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
            {@const isAuto = row.auto}
            {@const overridden = isOverridden(row)}
            <tr class="group hover:bg-base-300 divide-x divide-base-content/10"
                hidden={!showAutoHeaders && isAuto}>
                <td>
                    <input type="checkbox" class="checkbox checkbox-xs"
                        checked={row.enabled}
                        onchange={() => toggleRow(i)} />
                </td>
                <td>
                    <div
                        class="w-full"
                        class:tooltip={overridden}
                        data-tip={overridden ? "This header is overridden by your custom header" : undefined}>
                        <input
                            class="input input-ghost input-xs w-full font-mono p-0"
                            placeholder="Key"
                            value={row.key}
                            disabled={isAuto}
                            oninput={(e) => updateRow(i, "key", (e.target as HTMLInputElement).value)}
                        />
                    </div>
                </td>
                <td>
                    <div
                        class="w-full"
                        class:tooltip={overridden}
                        data-tip={overridden ? "This header is overridden by your custom header" : undefined}>
                        <input
                            class="input input-ghost input-xs w-full font-mono p-0"
                            class:line-through={overridden}
                            placeholder="Value"
                            value={row.value}
                            disabled={isAuto}
                            oninput={(e) => updateRow(i, "value", (e.target as HTMLInputElement).value)}
                        />
                    </div>
                </td>
                <td class="">
                    {#if !isAuto}
                        <DeleteRowButton onclick={() => removeRow(i)} />
                    {/if}
                </td>
            </tr>
        {/each}
        <tr>
            <td></td>
            <td class="p-0">
                <AddRowButton onclick={addRow} text="Add header" />
            </td>
        </tr>
    </tbody>
</table>
