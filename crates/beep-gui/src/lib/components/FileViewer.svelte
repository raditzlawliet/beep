<script lang="ts">
    import FileEditor from "$lib/components/FileEditor.svelte";

    interface Props {
        fileName: string;
        content: string;
        onContentChange?: (newContent: string) => void;
    }

    let { fileName, content, onContentChange }: Props = $props();

    function detectLanguage(name: string, raw: string): "text" | "json" | "html" | "xml" {
        const ext = name.split(".").pop()?.toLowerCase();
        if (ext === "json") return "json";
        if (ext === "html" || ext === "htm") return "html";
        if (ext === "xml") return "xml";

        const trimmed = raw.trim();
        if ((trimmed.startsWith("{") || trimmed.startsWith("[")) && trimmed.length > 0) {
            try {
                JSON.parse(trimmed);
                return "json";
            } catch { /* not valid JSON */ }
        }
        return "text";
    }

    let language = $derived(detectLanguage(fileName, content));
</script>

<div class="flex flex-col h-full">
    <FileEditor
        value={content}
        {language}
        wrapLines={true}
        onchange={(v) => onContentChange?.(v)}
        class="flex-1 min-h-0"
    />
</div>
