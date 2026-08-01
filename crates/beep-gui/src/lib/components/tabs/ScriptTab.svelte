<script lang="ts">
    import CodeEditor from "$lib/components/CodeEditor.svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { UploadIcon, XIcon } from "@lucide/svelte";

    interface Props {
        value: string | null;
        external: boolean;
        onchange?: (value: string | null, isExternal: boolean) => void;
        basePath?: string | null;
        /** Description shown below the file picker, e.g. "Script will be run before request is sent." */
        description?: string;
        /** Emit on each input change with a debounce (true) or only on blur (false). */
        debouncedChange?: boolean;
    }

    let {
        value,
        external = false,
        onchange,
        basePath = null,
        description = "",
        debouncedChange = false,
    }: Props = $props();

    function resolvePath(absolutePath: string): string {
        if (!basePath) return absolutePath;
        const normalizedBase = basePath.replace(/\\/g, "/").replace(/\/+$/, "");
        const normalizedAbs = absolutePath.replace(/\\/g, "/");
        // Only compare case-insensitively on Windows
        const isWindows = typeof navigator !== "undefined" && /win/i.test(navigator.userAgent ?? "");
        if (isWindows
            ? normalizedAbs.toLowerCase().startsWith(normalizedBase.toLowerCase() + "/")
            : normalizedAbs.startsWith(normalizedBase + "/")
        ) {
            return "./" + absolutePath.slice(normalizedBase.length + 1).replace(/\\/g, "/");
        }
        return absolutePath;
    }

    let useExternalFile = $state(false);
    let scriptCode = $state("");
    let filePath = $state("");
    let lastEmitted = $state<{ val: string | null; ext: boolean }>({ val: null, ext: false });

    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    $effect(() => {
        const ext = external;
        const val = value;
        if (lastEmitted.val === val && lastEmitted.ext === ext) return;
        useExternalFile = ext;
        if (ext) {
            filePath = val ?? "";
            scriptCode = "";
        } else {
            scriptCode = val ?? "";
            filePath = "";
        }
        lastEmitted = { val, ext };
    });

    function emit() {
        const ext = useExternalFile;
        const val = ext ? (filePath || null) : (scriptCode || null);
        lastEmitted = { val, ext };
        onchange?.(val, ext);
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
        try {
            const selected = await open({
                multiple: false,
                title: "Select JavaScript file",
                filters: [
                    { name: "JavaScript", extensions: ["js", "mjs"] },
                    { name: "All files", extensions: ["*"] },
                ],
            });
            if (selected && typeof selected === "string") {
                filePath = resolvePath(selected);
                emit();
            }
        } catch (_) {
            // User cancelled or platform error - silently ignore
        }
    }

    function handleCodeChange(v: string) {
        scriptCode = v;
        if (!debouncedChange) return;
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
            emit();
        }, 500);
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
                <button
                    class="btn btn-ghost btn-xs text-error shrink-0"
                    aria-label="Clear file path"
                    onclick={() => { filePath = ""; emit(); }}
                >
                    <XIcon class="w-3 h-3" />
                </button>
            {:else}
                <button class="btn btn-ghost btn-xs font-normal gap-1 w-full"
                    onclick={pickFile}>
                    <UploadIcon class="w-3 h-3" /> Choose file
                </button>
            {/if}
        </div>

        {#if description}
            <p class="text-xs text-base-content/50">{@html description}</p>
        {/if}
    {:else}
        <div class="flex-1 min-h-0 border border-base-content/10 rounded-lg overflow-hidden">
            <CodeEditor
                value={scriptCode}
                language="javascript"
                readonly={false}
                onchange={debouncedChange ? handleCodeChange : undefined}
                onblur={(v) => {
                    scriptCode = v;
                    emit();
                }}
            />
        </div>
    {/if}
</div>
