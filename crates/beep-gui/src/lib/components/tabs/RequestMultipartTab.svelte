<script lang="ts">
    import type { ParsedFormField } from "$lib/types";
    import { ChevronDownIcon, CheckIcon, PaperclipIcon, UploadIcon, XIcon } from "@lucide/svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";
    import { open } from "@tauri-apps/plugin-dialog";

    interface Props {
        initialValue: ParsedFormField[];
        basePath: string | null;
        onchange: (fields: ParsedFormField[]) => void;
    }

    let { initialValue = [], basePath = null, onchange }: Props = $props();

    type Row = {
        key: string;
        value: string;
        enabled: boolean;
        fieldType: "text" | "file";
        contentType: string;
        autoContentType: boolean;
        fileName: string;
        isInline: boolean;
    };
    let rows = $state<Row[]>([]);
    let dragOverIdx = $state<number | null>(null);

    const CONTENT_TYPES = [
        "application/octet-stream",
        "application/json",
        "application/xml",
        "application/pdf",
        "application/zip",
        "text/plain",
        "text/csv",
        "text/html",
        "image/png",
        "image/jpeg",
        "image/gif",
        "image/svg+xml",
        "audio/mpeg",
        "video/mp4",
    ];

    // Content-type combobox state
    let ctOpenIdx = $state<number | null>(null);
    let ctFilter = $state("");
    let ctTrigger = $state<HTMLElement | null>(null);
    let ctHighlight = $state(0);
    let ctListEl = $state<HTMLElement | null>(null);

    // Auto-scroll highlighted item into view on keyboard navigation
    $effect(() => {
        if (ctOpenIdx !== null && ctListEl && ctHighlight >= 0) {
            const items = ctListEl.querySelectorAll("li button");
            if (ctHighlight < items.length) {
                items[ctHighlight].scrollIntoView({ block: "nearest" });
            }
        }
    });

    const filteredCt = $derived(
        ctFilter
            ? CONTENT_TYPES.filter((ct) => ct.toLowerCase().includes(ctFilter.toLowerCase()))
            : CONTENT_TYPES,
    );

    function ctItemCount(): number {
        let n = filteredCt.length;
        if (ctFilter && !CONTENT_TYPES.includes(ctFilter)) n += 1; // custom entry
        return n;
    }

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
            // None = not set (checkbox off, no Content-Type)
            // Some("") = auto (checkbox on, Content-Type with empty value)
            // Some("val") = explicit (checkbox off, typed value)
            contentType: f.content_type ?? "",
            autoContentType: f.content_type === "",
            fileName: f.field_type === "file" && f.value
                ? f.value.split(/[/\\]/).pop() ?? f.value
                : "",
            isInline: f.is_inline,
        }));
    }

    function emit() {
        const out: ParsedFormField[] = [];
        for (const r of rows) {
            const ct = r.autoContentType
                ? ""  // auto -> Content-Type: (empty value)
                : (r.contentType || null); // null = no Content-Type, string = explicit
            out.push({
                key: r.key.trim(),
                value: r.value,
                enabled: r.enabled,
                field_type: r.fieldType,
                content_type: ct,
                is_inline: r.isInline,
            });
        }
        onchange(out);
    }

    function updateRow(idx: number, field: "key" | "value", val: string) {
        const r = rows[idx];
        rows[idx] = { ...r, [field]: val };
        emit();
    }

    function setContentType(idx: number, val: string) {
        const r = rows[idx];
        rows[idx] = { ...r, contentType: val };
        emit();
    }

    function openCtDropdown(idx: number, el: HTMLElement) {
        ctOpenIdx = idx;
        ctFilter = rows[idx].contentType;
        ctTrigger = el;
    }

    function closeCtDropdown() {
        setTimeout(() => {
            ctOpenIdx = null;
            ctFilter = "";
            ctTrigger = null;
        }, 150);
    }

    function handleCtFilter(e: Event, idx: number) {
        const v = (e.target as HTMLInputElement).value;
        ctFilter = v;
        ctHighlight = 0;
        setContentType(idx, v);
        if (ctOpenIdx === null) {
            ctOpenIdx = idx;
            ctTrigger = e.target as HTMLElement;
        }
    }

    function handleCtKeydown(e: KeyboardEvent) {
        if (ctOpenIdx === null) return;
        const count = ctItemCount();
        if (e.key === "ArrowDown") {
            e.preventDefault();
            ctHighlight = (ctHighlight + 1) % count;
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            ctHighlight = (ctHighlight - 1 + count) % count;
        } else if (e.key === "Enter") {
            e.preventDefault();
            if (ctHighlight < filteredCt.length) {
                selectCt(filteredCt[ctHighlight]);
            } else if (ctFilter) {
                selectCt(ctFilter);
            }
        } else if (e.key === "Escape") {
            ctOpenIdx = null;
            ctFilter = "";
            ctTrigger = null;
        }
    }

    function selectCt(val: string) {
        if (ctOpenIdx !== null) {
            setContentType(ctOpenIdx, val);
            ctOpenIdx = null;
            ctFilter = "";
            ctTrigger = null;
        }
    }

    function toggleContentTypeAuto(idx: number) {
        const r = rows[idx];
        rows[idx] = { ...r, autoContentType: !r.autoContentType };
        emit();
    }

    function setFieldType(idx: number, ft: "text" | "file") {
        const r = rows[idx];
        rows[idx] = { ...r, fieldType: ft };
        emit();
    }

    function removeRow(idx: number) {
        rows.splice(idx, 1);
        emit();
    }

    function addRow() {
        rows = [...rows, { key: "", value: "", enabled: true, fieldType: "text", contentType: "", autoContentType: false, fileName: "", isInline: true }];
        emit();
    }

    function toggleRow(idx: number) {
        const r = rows[idx];
        rows[idx] = { ...r, enabled: !r.enabled };
        emit();
    }

    async function pickFile(idx: number) {
        const selected = await open({
            multiple: false,
            title: "Select file to upload",
        });
        if (selected && typeof selected === "string") {
            const r = rows[idx];
            // Convert to relative path if possible, fallback to absolute
            const resolvedPath = resolvePath(selected);
            const name = resolvedPath.split(/[/\\]/).pop() ?? resolvedPath;
            rows[idx] = { ...r, value: resolvedPath, fileName: name };
            emit();
        }
    }

    function resolvePath(absolutePath: string): string {
        if (!basePath) return absolutePath;
        // Normalize separators for comparison
        const normalizedBase = basePath.replace(/\\/g, "/").replace(/\/+$/, "");
        const normalizedAbs = absolutePath.replace(/\\/g, "/");
        if (normalizedAbs.startsWith(normalizedBase + "/")) {
            return "./" + normalizedAbs.slice(normalizedBase.length + 1);
        }
        return absolutePath;
    }

    // DnD handlers
    function onDragOver(e: DragEvent, idx: number) {
        if (rows[idx].fieldType !== "file") return;
        e.preventDefault();
        dragOverIdx = idx;
    }

    function onDragLeave() {
        dragOverIdx = null;
    }

    function onDrop(e: DragEvent, idx: number) {
        e.preventDefault();
        dragOverIdx = null;
        if (rows[idx].fieldType !== "file") return;

        const files = e.dataTransfer?.files;
        if (!files || files.length === 0) return;
        const file = files[0];
        // Tauri exposes file path on dropped files via __TAURI__ internals
        // Fallback: use file.name as value
        const absPath = (file as any).path ?? file.name;
        const resolvedPath = resolvePath(absPath);
        const r = rows[idx];
        rows[idx] = { ...r, value: resolvedPath, fileName: file.name };
        if (!r.contentType && file.type) {
            rows[idx].contentType = file.type;
        }
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
            <th class="w-60 text-xs">Content Type</th>
            <th class="w-0"></th>
        </tr>
    </thead>
    <tbody>
        {#each rows as row, i}
            <tr
                class="group hover:bg-base-300 divide-x divide-base-content/10 {dragOverIdx === i ? 'bg-accent/10' : ''}"
                ondragover={(e) => onDragOver(e, i)}
                ondragleave={onDragLeave}
                ondragend={onDragLeave}
                ondrop={(e) => onDrop(e, i)}
            >
                <td>
                    <input type="checkbox" class="checkbox checkbox-xs"
                        checked={row.enabled}
                        onchange={() => toggleRow(i)} />
                </td>
                <td class="flex items-center gap-1">
                    <input class="input input-ghost input-xs flex-1 font-mono p-0"
                        placeholder="Key" value={row.key}
                        oninput={(e) => updateRow(i, "key", (e.target as HTMLInputElement).value)} />
                    <div class="dropdown">
                        <button class="btn btn-ghost btn-xs font-normal gap-1 shrink-0" role="menu" tabindex="0">
                            {row.fieldType === "file" ? "File" : "Text"}
                            <ChevronDownIcon class="w-2.5 h-2.5" />
                        </button>
                        <ul class="dropdown-content menu menu-sm bg-base-200 rounded-box z-1 shadow-sm border border-base-content/10 w-20 p-1 gap-0.5" tabindex="-1">
                            <li>
                                <button onclick={() => { setFieldType(i, "text"); (document.activeElement as HTMLElement)?.blur(); }}>
                                    Text
                                    <CheckIcon class="w-3 h-3 ml-auto {row.fieldType === "text" ? '' : 'invisible'}" />
                                </button>
                            </li>
                            <li>
                                <button onclick={() => { setFieldType(i, "file"); (document.activeElement as HTMLElement)?.blur(); }}>
                                    File
                                    <CheckIcon class="w-3 h-3 ml-auto {row.fieldType === "file" ? '' : 'invisible'}" />
                                </button>
                            </li>
                        </ul>
                    </div>
                </td>
                <td>
                    {#if row.fieldType === "text"}
                        <input class="input input-ghost input-xs w-full font-mono p-0"
                            placeholder="Value" value={row.value}
                            oninput={(e) => updateRow(i, "value", (e.target as HTMLInputElement).value)} />
                    {:else}
                        <div class="flex items-center gap-1">
                            {#if row.fileName}
                                <PaperclipIcon class="w-3 h-3 opacity-50 shrink-0" />
                                <span class="text-xs truncate">{row.fileName}</span>
                            {:else}
                                <button class="btn btn-ghost btn-xs font-normal gap-1"
                                    onclick={() => pickFile(i)}>
                                    <UploadIcon class="w-3 h-3" /> Choose file
                                </button>
                            {/if}
                            {#if row.fileName}
                                <button class="btn btn-ghost btn-xs text-error shrink-0"
                                    onclick={() => { const r = rows[i]; rows[i] = { ...r, value: "", fileName: "", contentType: "" }; emit(); }}>
                                    <XIcon class="w-3 h-3" />
                                </button>
                            {/if}
                        </div>
                    {/if}
                </td>
                <td>
                    <div class="flex items-center gap-1">
                        <div class="relative flex-1">
                            <input
                                class="input input-ghost input-xs w-full font-mono p-0"
                                placeholder={row.autoContentType ? "(Auto)" : "Not set"}
                                value={row.contentType}
                                disabled={row.autoContentType}
                                onfocus={(e) => {
                                    if (!row.autoContentType) {
                                        ctHighlight = 0;
                                        openCtDropdown(i, e.target as HTMLElement);
                                    }
                                }}
                                onblur={closeCtDropdown}
                                oninput={(e) => handleCtFilter(e, i)}
                                onkeydown={handleCtKeydown}
                            />
                            {#if ctOpenIdx === i && ctTrigger}
                                {@const rect = ctTrigger.getBoundingClientRect()}
                                <ul
                                    class="fixed z-101 mt-1 bg-base-200 border border-base-content/20 rounded-box shadow-lg max-h-48 overflow-y-auto"
                                    style="top: {rect.bottom}px; left: {rect.left}px; width: {rect.width}px;"
                                    bind:this={ctListEl}
                                >
                                    {#each filteredCt as ct, j}
                                        {@const selected = rows[i].contentType === ct}
                                        <li>
                                            <button
                                                class="w-full text-left px-2 py-0.5 text-xs font-mono hover:bg-base-300 flex items-center gap-1 {selected ? 'bg-base-300' : ''} {ctHighlight === j ? 'outline outline-primary' : ''}"
                                                onmousedown={() => selectCt(ct)}
                                            >
                                                <span class="flex-1">{ct}</span>
                                                {#if selected}
                                                    <CheckIcon class="w-3 h-3 opacity-50 shrink-0" />
                                                {/if}
                                            </button>
                                        </li>
                                    {/each}
                                    {#if ctFilter && !CONTENT_TYPES.includes(ctFilter)}
                                        <li>
                                            <button
                                                class="w-full text-left px-2 py-0.5 text-xs font-mono hover:bg-base-300 flex items-center gap-1 bg-base-300 {ctHighlight === filteredCt.length ? 'outline outline-primary' : ''}"
                                                onmousedown={() => selectCt(ctFilter)}
                                            >
                                                <span class="flex-1">{ctFilter}</span>
                                                <CheckIcon class="w-3 h-3 opacity-50 shrink-0" />
                                            </button>
                                        </li>
                                    {/if}
                                </ul>
                            {/if}
                        </div>
                        <input
                            type="checkbox"
                            class="checkbox checkbox-xs shrink-0"
                            checked={row.autoContentType}
                            onchange={() => toggleContentTypeAuto(i)}
                            title="Auto content-type"
                        />
                    </div>
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
