<script lang="ts">
    import XmlTreeNode from "$lib/components/XmlTreeNode.svelte";

    interface Props {
        xml: string;
    }

    let { xml }: Props = $props();

    interface XmlNode {
        tag: string;
        attrs: Record<string, string>;
        children: XmlNode[];
        text: string;
    }

    type ParseResult = { tree: XmlNode } | { error: string };

    let result = $derived.by((): ParseResult => {
        try {
            const parser = new DOMParser();
            const doc = parser.parseFromString(xml, "text/xml");
            const err = doc.querySelector("parsererror");
            if (err) return { error: err.textContent || "Unknown parse error" };
            return { tree: nodeFromElement(doc.documentElement) };
        } catch (e) {
            return { error: String(e) };
        }
    });

    function nodeFromElement(el: Element): XmlNode {
        const attrs: Record<string, string> = {};
        for (const a of el.attributes) {
            attrs[a.name] = a.value;
        }
        const children: XmlNode[] = [];
        let text = "";
        for (const child of el.childNodes) {
            if (child.nodeType === Node.ELEMENT_NODE) {
                children.push(nodeFromElement(child as Element));
            } else if (child.nodeType === Node.TEXT_NODE) {
                const t = (child as Text).textContent?.trim() || "";
                if (t) text += t;
            }
        }
        const displayText = children.length === 0 ? text : "";
        return { tag: el.localName, attrs, children, text: displayText };
    }
</script>

{#if "tree" in result}
    <div class="overflow-auto h-full font-mono text-xs leading-relaxed p-2 select-none">
        <XmlTreeNode node={result.tree} depth={0} />
    </div>
{:else}
    <div class="flex items-center justify-center h-full p-4">
        <div class="text-xs text-center max-w-md">
            <p class="font-semibold mb-1">XML Parse Error</p>
            <p class="opacity-70 whitespace-pre-wrap break-all">{result.error}</p>
        </div>
    </div>
{/if}
