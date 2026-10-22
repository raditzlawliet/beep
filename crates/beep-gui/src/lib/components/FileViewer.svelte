<script lang="ts">
    import FileEditor from "$lib/components/FileEditor.svelte";
    import { languageFromName, sniffLanguage, type EditorLanguage } from "$lib/language-sniff";

    interface Props {
        fileName: string;
        content: string;
        onContentChange?: (newContent: string) => void;
        initialCursorPos?: number;
        oncursorchange?: (pos: number) => void;
    }

    let { fileName, content, onContentChange, initialCursorPos, oncursorchange }: Props = $props();

    // Detect once per file: extension first, else sniff content. While still unknown,
    // keep sniffing as the user types; lock on the first hit.
    let language = $state<EditorLanguage>("text");
    let _lockedFor: string | null = null;

    $effect(() => {
        const name = fileName;
        const raw = content;
        if (_lockedFor === name) return;
        const lang = languageFromName(name) ?? sniffLanguage(raw);
        language = lang ?? "text";
        if (lang) _lockedFor = name;
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
