<script lang="ts">
    import type { ProjectNode } from "$lib/types";
    import { Folder, File, FolderOpen } from "@lucide/svelte";

    interface Props {
        tree: ProjectNode[];
        projectName: string;
        activeFilePath: string | null;
        expanded: Set<string>;
        onToggleDir: (path: string) => void;
        onFileSelect: (node: ProjectNode) => void;
        onOpenProject: () => void;
    }

    let { tree, projectName, activeFilePath, expanded, onToggleDir, onFileSelect, onOpenProject }: Props = $props();

    let rootNode = $derived<ProjectNode>({
        name: projectName,
        path: tree.length > 0 ? tree[0].path.replace(/[/\\][^/\\]*$/, '') : '',
        is_dir: true,
        children: tree,
    });
</script>

<div class="flex flex-col h-full border-r border-base-300 w-full">
    {#if !projectName}
        <div class="flex-1 flex flex-col items-center justify-center gap-3 px-4">
            <span class="text-xs opacity-50">Open a folder to use the project panel</span>
            <button class="btn btn-ghost btn-sm" onclick={onOpenProject}>
                Open Project
            </button>
        </div>
    {:else}
        <div class="flex-1 overflow-y-auto py-1">
            {@render TreeNode({ node: rootNode, expanded, onToggleDir, activeFilePath, onFileSelect, depth: 0 })}
        </div>
    {/if}
</div>

{#snippet TreeNode(props: { node: ProjectNode; expanded: Set<string>; onToggleDir: (path: string) => void; activeFilePath: string | null; onFileSelect: (node: ProjectNode) => void; depth: number })}
    {@const { node, expanded, onToggleDir, activeFilePath, onFileSelect, depth } = props}
    {@const isOpen = expanded.has(node.path)}
    {@const isActive = activeFilePath === node.path}
    {@const padLeft = depth * 20 + 8}

    {#if node.is_dir}
        <div class="relative">
            <!-- guide lines for previous depth levels -->
            {#if depth > 0}
                {#each Array(depth) as _, d}
                    <div
                        class="absolute top-0 bottom-0 w-px bg-base-content/10 pointer-events-none"
                        style="left: {d * 20 + 14}px"
                    ></div>
                {/each}
            {/if}
        <button
            class="flex items-center w-full text-left text-xs py-1 hover:bg-base-200/50"
            style="padding-left: {padLeft}px"
            onclick={() => onToggleDir(node.path)}
        >
            {#if isOpen}
                <FolderOpen class="h-3.5 w-3.5 me-1.5 fill-base-content/40"/>
            {:else}
                <Folder class="h-3.5 w-3.5 me-1.5 opacity-70" />
            {/if}
            <span class="truncate">{node.name}</span>
        </button>
        </div>

        {#if isOpen && node.children}
            {#each node.children as child (child.path)}
                {@render TreeNode({ node: child, expanded, onToggleDir, activeFilePath, onFileSelect, depth: depth + 1 })}
            {/each}
        {/if}
    {:else}
        <div class="relative">
            <!-- guide lines for previous depth levels -->
            {#if depth > 0}
                {#each Array(depth) as _, d}
                    <div
                        class="absolute top-0 bottom-0 w-px bg-base-content/10 pointer-events-none"
                        style="left: {d * 20 + 14}px"
                    ></div>
                {/each}
            {/if}
        <button
            class="flex items-center w-full text-left text-xs py-1 hover:bg-base-200/50"
            class:bg-base-200={isActive}
            style="padding-left: {padLeft + 0}px"
            onclick={() => onFileSelect(node)}
        >
            <File class="h-3.5 w-3.5 me-1.5 fill-base-content/40" />
            <span class="truncate">{node.name}</span>
        </button>
        </div>
    {/if}
{/snippet}
