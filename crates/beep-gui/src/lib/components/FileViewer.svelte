<script lang="ts">
    import FileEditor from "$lib/components/FileEditor.svelte";
    import type { EditorLanguage } from "$lib/types";
    import { languageFromName, sniffLanguage } from "$lib/language-sniff";

    interface Props {
        fileName: string;
        content: string;
        /** Tab language; "text" = Plain Text. */
        language: EditorLanguage;
        /** User picked the language from the status bar - never guess again. */
        languageManual?: boolean;
        onLanguageDetected?: (lang: EditorLanguage) => void;
        onContentChange?: (newContent: string) => void;
        initialCursorPos?: number;
        oncursorchange?: (pos: number) => void;
    }

    let {
        fileName,
        content,
        language,
        languageManual = false,
        onLanguageDetected,
        onContentChange,
        initialCursorPos,
        oncursorchange,
    }: Props = $props();

    // Guess only while Plain Text and not picked by hand: extension first, else sniff.
    // Still unknown -> try again as the user types. First hit sticks on the tab.
    $effect(() => {
        if (languageManual || language !== "text") return;
        const lang = languageFromName(fileName) ?? sniffLanguage(content);
        if (lang && lang !== "text") onLanguageDetected?.(lang);
    });
</script>

<div class="flex flex-col h-full">
    <FileEditor
        value={content}
        {language}
        wrapLines={true}
        onchange={(v) => onContentChange?.(v)}
        {initialCursorPos}
        {oncursorchange}
        class="flex-1 min-h-0"
    />
</div>
