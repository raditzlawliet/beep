<script lang="ts">
    import XmlTreeNode from "./XmlTreeNode.svelte";
    import { ChevronRightIcon, ChevronDownIcon } from "@lucide/svelte";

    interface XmlNode {
        tag: string;
        attrs: Record<string, string>;
        children: XmlNode[];
        text: string;
    }

    interface Props {
        node: XmlNode;
        depth: number;
    }

    let { node, depth }: Props = $props();

    let expanded = $state(true);

    const hasKids = $derived(node.children.length > 0);
    const isEmpty = $derived(!hasKids && !node.text);

    function toggle() {
        if (hasKids) expanded = !expanded;
    }
</script>

<div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        style="padding-left: {depth * 2}ch"
        class="flex items-start gap-0 hover:bg-base-200/50 rounded px-0.5 cursor-default whitespace-nowrap"
        role="button"
        tabindex={hasKids ? 0 : -1}
        onclick={toggle}
        onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                toggle();
            }
        }}
    >
        <!-- arrow -->
        <span style="width:2ch" class="shrink-0 self-stretch inline-flex items-center justify-center">
            {#if hasKids}
                {#if expanded}
                    <ChevronDownIcon class="w-3 h-3 opacity-50" />
                {:else}
                    <ChevronRightIcon class="w-3 h-3 opacity-50" />
                {/if}
            {/if}
        </span>

        <span class="text-accent">&lt;</span><span class="text-warning">{node.tag}</span>

        {#each Object.entries(node.attrs) as [key, value]}
            <span style="width:1ch" class="shrink-0"></span>
            <span class="text-info">{key}</span><span class="opacity-40">=</span><span class="text-success">"</span><span class="text-success opacity-80">{value}</span><span class="text-success">"</span>
        {/each}

        {#if isEmpty}
            <span class="text-accent">/&gt;</span>
        {:else if !hasKids}
            <span class="text-accent">&gt;</span><span class="text-base-content/80">{node.text}</span><span class="text-accent">&lt;/</span><span class="text-warning">{node.tag}</span><span class="text-accent">&gt;</span>
        {:else if !expanded}
            <span class="text-accent">&gt;</span><span class="opacity-30">…</span><span class="text-accent">&lt;/</span><span class="text-warning">{node.tag}</span><span class="text-accent">&gt;</span>
        {:else}
            <span class="text-accent">&gt;</span>
        {/if}
    </div>

    {#if hasKids && expanded}
        {#each node.children as child}
            <XmlTreeNode node={child} depth={depth + 1} />
        {/each}
    {/if}

    {#if hasKids && expanded}
        <div style="padding-left: {depth * 2}ch" class="flex items-start gap-0 whitespace-nowrap">
            <span style="width:2ch" class="shrink-0"></span>
            <span class="text-accent">&lt;/</span><span class="text-warning">{node.tag}</span><span class="text-accent">&gt;</span>
        </div>
    {/if}
</div>
