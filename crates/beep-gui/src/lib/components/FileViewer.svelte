<script lang="ts">
    import FileEditor from "$lib/components/FileEditor.svelte";
    import VarsFileTable from "$lib/components/VarsFileTable.svelte";
    import type { EditorLanguage } from "$lib/types";
    import { languageFromName, sniffLanguage } from "$lib/language-sniff";
    import { beepJsonKind, envNameOf } from "$lib/vars/beep-files";
    import { environment } from "$lib/app-state.svelte";
    import { setModeHandler } from "$lib/hotkeys.svelte";

    interface Props {
        fileName: string;
        /** Full path; `.beep` env/vars JSON files get the table view. */
        filePath?: string;
        content: string;
        /** Tab language; "text" = Plain Text. */
        language: EditorLanguage;
        /** User picked the language from the status bar - never guess again. */
        languageManual?: boolean;
        /** `.beep` JSON files: "file" = form (default), "code" = JSON text. */
        beepView?: "code" | "file";
        onBeepViewChange?: (view: "code" | "file") => void;
        onLanguageDetected?: (lang: EditorLanguage) => void;
        onContentChange?: (newContent: string) => void;
        initialCursorPos?: number;
        oncursorchange?: (pos: number) => void;
    }

    let {
        fileName,
        filePath,
        content,
        language,
        languageManual = false,
        beepView = "file",
        onBeepViewChange,
        onLanguageDetected,
        onContentChange,
        initialCursorPos,
        oncursorchange,
    }: Props = $props();

    const kind = $derived(filePath ? beepJsonKind(filePath) : null);
    const envName = $derived(kind === "env" && filePath ? envNameOf(filePath) : null);
    const envActive = $derived(kind === "env" && environment.selected === envName);

    // Ctrl+1 / Ctrl+2 = Code / File on `.beep` env/vars tabs.
    $effect(() => {
        if (!kind) return;
        setModeHandler((mode) => {
            if (mode === "code" || mode === "file") onBeepViewChange?.(mode);
        });
        return () => setModeHandler(null);
    });

    // Guess only while Plain Text and not picked by hand: extension first, else sniff.
    // Still unknown -> try again as the user types. First hit sticks on the tab.
    $effect(() => {
        if (languageManual || language !== "text") return;
        const lang = languageFromName(fileName) ?? sniffLanguage(content);
        if (lang && lang !== "text") onLanguageDetected?.(lang);
    });
</script>

<div class="flex flex-col h-full">
    {#if kind}
        <!-- fixed h-8: the Active badge and the "Use this environment" button differ in height -->
        <div class="flex items-center gap-2 px-2 h-8 shrink-0 border-b border-base-content/10 text-xs">
            {#if kind === "env"}
                <span class="opacity-70">Environment: <b>{envName ?? "Default"}</b></span>
                {#if envActive}
                    <span class="badge badge-xs badge-primary">Active</span>
                {:else}
                    <button class="btn btn-xs btn-ghost"
                        onclick={() => environment.select(envName).catch((e) => console.error("select env", e))}>
                        Use this environment
                    </button>
                {/if}
            {:else}
                <span class="opacity-70">Collection vars</span>
            {/if}
        </div>
    {/if}
    {#if kind && beepView === "file"}
        <div class="flex-1 min-h-0 overflow-auto">
            <VarsFileTable {content}
                onchange={(v) => onContentChange?.(v)}
                onShowCode={() => onBeepViewChange?.("code")} />
        </div>
    {:else}
        <FileEditor
            value={content}
            {language}
            wrapLines={true}
            onchange={(v) => onContentChange?.(v)}
            {initialCursorPos}
            {oncursorchange}
            class="flex-1 min-h-0"
        />
    {/if}
</div>
