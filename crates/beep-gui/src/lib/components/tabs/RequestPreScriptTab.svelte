<script lang="ts">
    import CodeEditor from "$lib/components/CodeEditor.svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { UploadIcon, XIcon } from "@lucide/svelte";

    interface Props {
        value: string | null;
        external: boolean;
        onchange?: (value: string | null, isExternal: boolean) => void;
        basePath?: string | null;
    }

    let { value, external = false, onchange, basePath = null }: Props = $props();

    function resolvePath(absolutePath: string): string {
        if (!basePath) return absolutePath;
        const normalizedBase = basePath.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
        const normalizedAbs = absolutePath.replace(/\\/g, "/").toLowerCase();
        if (normalizedAbs.startsWith(normalizedBase + "/")) {
            return "./" + absolutePath.slice(normalizedBase.length + 1).replace(/\\/g, "/");
        }
        return absolutePath;
    }

    let useExternalFile = $state(false);
    let scriptCode = $state("");
    let filePath = $state("");
    let internalChange = false;

    $effect(() => {
        if (internalChange) {
            internalChange = false;
            return;
        }
        useExternalFile = external;
        if (external) {
            filePath = value ?? "";
            scriptCode = "";
        } else {
            scriptCode = value ?? "";
            filePath = "";
        }
    });

    function emit() {
        internalChange = true;
        if (useExternalFile) {
            onchange?.(filePath || null, true);
        } else {
            onchange?.(scriptCode || null, false);
        }
    }

    function toggleMode() {
        useExternalFile = !useExternalFile;
        if (useExternalFile) {
            scriptCode = "";
        } else {
            filePath = "";
        }
        emit();
    }

    async function pickFile() {
        const selected = await open({
            multiple: false,
            title: "Select JavaScript file",
            filters: [{ name: "JavaScript", extensions: ["js", "mjs", "*"] }],
        });
        if (selected && typeof selected === "string") {
            filePath = resolvePath(selected);
            internalChange = true;
            onchange?.(filePath || null, true);
        }
    }
</script>

<div class="flex flex-col h-full p-2 gap-2 min-h-0">
    <!-- Mode toggle -->
    <label class="flex items-center gap-2 cursor-pointer text-sm">
        <input
            type="checkbox"
            class="checkbox checkbox-xs"
            checked={useExternalFile}
            onchange={toggleMode}
        />
        <span>Use external file</span>
    </label>

    {#if useExternalFile}
        <div class="flex items-center gap-1">
            {#if filePath}
                <span class="text-xs font-mono truncate flex-1">{filePath}</span>
                <button class="btn btn-ghost btn-xs text-error shrink-0"
                    onclick={() => { filePath = ""; emit(); }}>
                    <XIcon class="w-3 h-3" />
                </button>
            {:else}
                <button class="btn btn-ghost btn-xs font-normal gap-1"
                    onclick={pickFile}>
                    <UploadIcon class="w-3 h-3" /> Choose file
                </button>
            {/if}
        </div>
    {:else}
        <div class="flex-1 min-h-0 border border-base-content/10 rounded-lg overflow-hidden">
            <CodeEditor
                value={scriptCode}
                language="javascript"
                readonly={false}
                onblur={(v) => {
                    scriptCode = v;
                    emit();
                }}
            />
        </div>
    {/if}
</div>
