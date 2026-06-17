<script lang="ts">
    import { EyeIcon, EyeOffIcon } from "@lucide/svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";

    interface Props {
        initialValue: Record<string, string>;
        defaultHeaders: [string, string][];
        onchange: (headers: Record<string, string>) => void;
    }

    let { initialValue = {}, defaultHeaders = [], onchange }: Props = $props();

    type Row = { key: string; value: string; enabled: boolean; auto: boolean };
    let rows = $state<Row[]>([]);
    let showAutoHeaders = $state(false);

    let _lastInit = $state("");
    let _prevDefaultHeaders: [string, string][] = [];

    // Init from props
    function initFromProps() {
        const dhChanged = _prevDefaultHeaders.length !== defaultHeaders.length ||
            _prevDefaultHeaders.some(([k, v], i) => defaultHeaders[i]?.[0] !== k || defaultHeaders[i]?.[1] !== v);
        const rhKey = JSON.stringify(initialValue);
        const rhChanged = rhKey !== _lastInit;

        if (!dhChanged && !rhChanged) return;

        _lastInit = rhKey;
        _prevDefaultHeaders = [...defaultHeaders];

        rows = [];

        // Default headers first (auto-generated)
        const existingKeys = new Set(Object.keys(initialValue).map((k) => k.toLowerCase()));
        for (const [key, value] of defaultHeaders) {
            if (!existingKeys.has(key.toLowerCase())) {
                rows.push({ key, value, enabled: true, auto: true });
            }
        }

        // User-defined headers
        for (const [key, value] of Object.entries(initialValue)) {
            rows.push({ key, value, enabled: true, auto: false });
        }
        ensureTempRow();
    }

    $effect(() => {
        void JSON.stringify(initialValue);
        void defaultHeaders;
        initFromProps();
    });

    // Row management
    function ensureTempRow() {
        const last = rows[rows.length - 1];
        if (rows.length === 0 || (last && (last.key.trim() || last.value.trim()))) {
            rows.push({ key: "", value: "", enabled: true, auto: false });
        }
    }

    function emit() {
        const obj: Record<string, string> = {};
        for (const r of rows) {
            if (r.enabled && r.key.trim() && !r.auto) obj[r.key.trim()] = r.value;
        }
        _lastInit = JSON.stringify(obj);
        onchange(obj);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const r = rows[idx];
        rows[idx] = { ...r, [field]: val };
        ensureTempRow();
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
            {@const isLast = i === rows.length - 1 && !row.key.trim() && !row.value.trim()}
            <tr class="group hover:bg-base-300 divide-x divide-base-content/10"
                class:opacity-50={isAuto}
                hidden={!showAutoHeaders && isAuto}>
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
                    <input class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Value" value={row.value}
                        oninput={(e) => updateRow(i, "value", (e.target as HTMLInputElement).value)} />
                </td>
                <td class="">
                    {#if !isLast}
                        <DeleteRowButton onclick={() => removeRow(i)} />
                    {/if}
                </td>
            </tr>
        {/each}
    </tbody>
</table>
