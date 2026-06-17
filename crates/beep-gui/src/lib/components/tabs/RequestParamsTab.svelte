<script lang="ts">
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";

    interface Props {
        initialValue: Record<string, string>;
        url: string;
        onchange: (params: Record<string, string>, url: string) => void;
    }

    let { initialValue = {}, url = "", onchange }: Props = $props();

    type Row = { key: string; value: string; enabled: boolean };
    let rows = $state<Row[]>([]);

    // Track last synced state to avoid loops.
    let _lastQp = $state("");
    let _lastUrl = $state("");

    // Init from props
    function initFromProps() {
        const qpKey = JSON.stringify(initialValue);
        const urlChanged = url !== _lastUrl;
        const qpChanged = qpKey !== _lastQp;

        if (!urlChanged && !qpChanged) return;

        if (qpChanged) {
            _lastQp = qpKey;
            rows = Object.entries(initialValue).map(([k, v]) => ({
                key: k,
                value: v,
                enabled: true,
            }));
            ensureTempRow();
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

    // Row management
    function ensureTempRow() {
        const last = rows[rows.length - 1];
        if (rows.length === 0 || (last && (last.key.trim() || last.value.trim()))) {
            rows.push({ key: "", value: "", enabled: true });
        }
    }

    function emit() {
        const obj: Record<string, string> = {};
        for (const r of rows) {
            if (r.enabled && r.key.trim()) obj[r.key.trim()] = r.value;
        }
        _lastQp = JSON.stringify(obj);
        onchange(obj, url);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const oldRows = rows.map((r) => ({ ...r }));
        const r = rows[idx];
        rows[idx] = { ...r, [field]: val };
        ensureTempRow();
        syncUrlFromParams(oldRows);
    }

    function removeRow(idx: number) {
        const oldRows = rows.map((r) => ({ ...r }));
        rows.splice(idx, 1);
        ensureTempRow();
        syncUrlFromParams(oldRows);
    }

    function toggleRow(idx: number) {
        const oldRows = rows.map((r) => ({ ...r }));
        const r = rows[idx];
        rows[idx] = { ...r, enabled: !r.enabled };
        syncUrlFromParams(oldRows);
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
                    out.push({ key: decodeURIComponent(part.slice(0, eq)), value: decodeURIComponent(part.slice(eq + 1)) });
                } else if (part.trim()) {
                    out.push({ key: decodeURIComponent(part), value: "" });
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
                emit(); onchange({}, newUrl);
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
                    emit(); onchange({}, newUrl);
                }
            }
            return;
        }

        // Fallback
        const realRows = newRows.filter((r) => r.key.trim() || r.value.trim());
        const newUrl = buildUrlWithParams(url, realRows);
        if (newUrl !== url) {
            _lastUrl = newUrl;
            emit(); onchange({}, newUrl);
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
        ensureTempRow();
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
