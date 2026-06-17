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

    type Row = { key: string; value: string; enabled: boolean };
    let rows = $state<Row[]>([]);

    let _lastQp = $state("");
    let _lastUrl = $state("");

    function initFromProps() {
        const qpKey = JSON.stringify(initialValue);
        const urlChanged = url !== _lastUrl;
        const qpChanged = qpKey !== _lastQp;

        if (!urlChanged && !qpChanged) return;

        if (qpChanged) {
            _lastQp = qpKey;
            rows = initialValue.map((q) => ({
                key: q.key,
                value: q.value,
                enabled: q.enabled,
            }));
        }

        if (urlChanged) {
            _lastUrl = url;
            syncTableFromUrl(url);
        }
    }

    $effect(() => {
        void JSON.stringify(initialValue);
        void url;
        initFromProps();
    });

    function emit() {
        const out: QueryField[] = rows.map((r) => ({
            key: r.key.trim(),
            value: r.value,
            enabled: r.enabled,
        }));
        _lastQp = JSON.stringify(out);
        onchange(out, _lastUrl || url);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const oldRows = rows.map((r) => ({ ...r }));
        const r = rows[idx];
        rows[idx] = { ...r, [field]: val };
        syncUrlFromParams(oldRows);
    }

    function removeRow(idx: number) {
        const oldRows = rows.map((r) => ({ ...r }));
        rows.splice(idx, 1);
        syncUrlFromParams(oldRows);
    }

    function addRow() {
        rows = [...rows, { key: "", value: "", enabled: true }];
        emit();
    }

    function toggleRow(idx: number) {
        const oldRows = rows.map((r) => ({ ...r }));
        const r = rows[idx];
        rows[idx] = { ...r, enabled: !r.enabled };
        syncUrlFromParams(oldRows);
    }

    function safeDecode(s: string): string {
        try { return decodeURIComponent(s); } catch { return s; }
    }

    // URL <-> table sync
    type UrlParam = { key: string; value: string };
    function parseUrlParamsOrdered(u: string): UrlParam[] {
        try {
            return [...new URL(u).searchParams].map(([k, v]) => ({ key: k, value: v }));
        } catch {
            const idx = u.indexOf("?");
            if (idx < 0) return [];
            const qs = u.slice(idx + 1);
            const out: UrlParam[] = [];
            for (const part of qs.split("&")) {
                const eq = part.indexOf("=");
                if (eq >= 0) {
                    out.push({ key: safeDecode(part.slice(0, eq)), value: safeDecode(part.slice(eq + 1)) });
                } else if (part.trim()) {
                    out.push({ key: safeDecode(part), value: "" });
                }
            }
            return out;
        }
    }

    function buildUrlFromOrdered(baseUrl: string, params: UrlParam[]): string {
        try {
            const u = new URL(baseUrl);
            u.search = "";
            for (const p of params) u.searchParams.append(p.key, p.value);
            return u.toString();
        } catch {
            const qs = params.map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`).join("&");
            const idx = baseUrl.indexOf("?");
            const base = idx >= 0 ? baseUrl.slice(0, idx) : baseUrl;
            return qs ? `${base}?${qs}` : base;
        }
    }

    function buildUrlWithParams(baseUrl: string, r: Row[]): string {
        try {
            const u = new URL(baseUrl);
            u.search = "";
            for (const row of r) {
                if (row.enabled && row.key.trim()) u.searchParams.append(row.key.trim(), row.value);
            }
            return u.toString();
        } catch {
            const qs = r
                .filter((row) => row.enabled && row.key.trim())
                .map((row) => `${encodeURIComponent(row.key.trim())}=${encodeURIComponent(row.value)}`)
                .join("&");
            const idx = baseUrl.indexOf("?");
            const base = idx >= 0 ? baseUrl.slice(0, idx) : baseUrl;
            return qs ? `${base}?${qs}` : base;
        }
    }

    function enabledFromRows(r: Row[]): (Row & { pos: number })[] {
        const out: (Row & { pos: number })[] = [];
        let pos = 0;
        for (const row of r) {
            if (row.enabled && row.key.trim()) {
                out.push({ ...row, pos });
                pos++;
            }
        }
        return out;
    }

    function syncUrlFromParams(oldRows: Row[]) {
        const newRows = rows;
        const oldEP = enabledFromRows(oldRows);
        const newEP = enabledFromRows(newRows);
        const urlP = parseUrlParamsOrdered(url);

        let aligned = oldEP.length === urlP.length;
        if (aligned) {
            for (let i = 0; i < oldEP.length; i++) {
                if (oldEP[i].key !== urlP[i].key || oldEP[i].value !== urlP[i].value) {
                    aligned = false; break;
                }
            }
        }

        if (aligned && oldEP.length !== newEP.length) {
            const newParams: UrlParam[] = newEP.map((r) => ({ key: r.key, value: r.value }));
            const newUrl = buildUrlFromOrdered(url, newParams);
            if (newUrl !== url) {
                _lastUrl = newUrl;
                emit();
            }
            return;
        }

        if (aligned && oldEP.length === newEP.length) {
            let changed = false;
            const newParams = urlP.slice();
            for (let i = 0; i < oldEP.length; i++) {
                if (oldEP[i].key !== newEP[i].key || oldEP[i].value !== newEP[i].value) {
                    newParams[i] = { key: newEP[i].key, value: newEP[i].value };
                    changed = true;
                }
            }
            if (changed) {
                const newUrl = buildUrlFromOrdered(url, newParams);
                if (newUrl !== url) {
                    _lastUrl = newUrl;
                    emit();
                }
            }
            return;
        }

        // Fallback
        const realRows = newRows.filter((r) => r.key.trim() || r.value.trim());
        const newUrl = buildUrlWithParams(url, realRows);
        if (newUrl !== url) {
            _lastUrl = newUrl;
            emit();
        }
    }

    function syncTableFromUrl(u: string) {
        const urlP = parseUrlParamsOrdered(u);
        const tableMap = new Map<string, Row>();
        for (const r of rows) {
            if (r.key.trim()) tableMap.set(r.key.trim(), r);
        }
        const newRows: Row[] = [];
        for (const p of urlP) {
            const existing = tableMap.get(p.key);
            newRows.push({ key: p.key, value: p.value, enabled: existing ? existing.enabled : true });
        }
        for (const r of rows) {
            if (!r.enabled && r.key.trim() && !urlP.some((p) => p.key === r.key.trim())) {
                newRows.push({ ...r });
            }
        }
        rows = newRows;
    }
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
                <td class="">
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
