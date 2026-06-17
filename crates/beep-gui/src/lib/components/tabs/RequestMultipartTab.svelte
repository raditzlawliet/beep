<script lang="ts">
    import type { FormField } from "$lib/types";
    import { ChevronDownIcon, CheckIcon, PaperclipIcon, UploadIcon, XIcon } from "@lucide/svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";
    import { open } from "@tauri-apps/plugin-dialog";

    interface Props {
        initialValue: FormField[];
        onchange: (fields: FormField[]) => void;
    }

    let { initialValue = [], onchange }: Props = $props();

    type Row = {
        key: string;
        value: string;
        enabled: boolean;
        fieldType: "text" | "file";
        contentType: string;
        fileName: string; // display name for file picker
    };
    let rows = $state<Row[]>([]);
    let dragOverIdx = $state<number | null>(null);

    let _lastInit = $state("");

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

    function initFromProps() {
        const key = JSON.stringify(initialValue);
        if (key === _lastInit) return;
        _lastInit = key;
        rows = initialValue.map((f) => ({
            key: f.key,
            value: f.value,
            enabled: f.enabled,
            fieldType: (f.field_type === "file" ? "file" : "text") as "text" | "file",
            contentType: f.content_type ?? "",
            fileName: f.field_type === "file" && f.value
                ? f.value.split(/[/\\]/).pop() ?? f.value
                : "",
        }));
    }

    function emit() {
        const out: FormField[] = [];
        for (const r of rows) {
            out.push({
                key: r.key.trim(),
                value: r.value,
                enabled: r.enabled,
                field_type: r.fieldType,
                content_type: r.contentType,
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
        rows = [...rows, { key: "", value: "", enabled: true, fieldType: "text", contentType: "", fileName: "" }];
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
            const name = selected.split(/[/\\]/).pop() ?? selected;
            rows[idx] = { ...r, value: selected, fileName: name };
            emit();
        }
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
        dragOverIdx = null;
        const files = e.dataTransfer?.files;
        if (!files || files.length === 0) return;
        const file = files[0];
        // Tauri exposes file path on dropped files via __TAURI__ internals
        // Fallback: use file.name as value
        const path = (file as any).path ?? file.name;
        const r = rows[idx];
        rows[idx] = { ...r, value: path, fileName: file.name };
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
            <th class="w-40 text-xs">Content Type</th>
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
                                    onclick={() => { const r = rows[i]; rows[i] = { ...r, value: "", fileName: "" }; emit(); }}>
                                    <XIcon class="w-3 h-3" />
                                </button>
                            {/if}
                        </div>
                    {/if}
                </td>
                <td>
                    <input
                        class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Auto"
                        value={row.contentType}
                        list="content-types"
                        oninput={(e) => setContentType(i, (e.target as HTMLInputElement).value)}
                    />
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

<datalist id="content-types">
    {#each CONTENT_TYPES as ct}
        <option value={ct}></option>
    {/each}
</datalist>
