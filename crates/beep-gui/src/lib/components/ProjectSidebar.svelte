<script lang="ts">
    import type { ProjectNode } from "$lib/types";
    import { Folder, File, FolderOpen, Loader } from "@lucide/svelte";

    interface Props {
        tree: ProjectNode[];
        projectName: string;
        activeFilePath: string | null;
        expanded: Set<string>;
        loadingDirs: Set<string>;
        onToggleDir: (path: string) => void;
        onFileSelect: (node: ProjectNode) => void;
        onFileDblClick: (node: ProjectNode) => void;
        onOpenProject: () => void;
    }

    let { tree, projectName, activeFilePath, expanded, loadingDirs, onToggleDir, onFileSelect, onFileDblClick, onOpenProject }: Props = $props();

    let rootNode = $derived<ProjectNode>({
        name: projectName,
        path: tree.length > 0 ? tree[0].path.replace(/[/\\][^/\\]*$/, '') : '',
        is_dir: true,
        children: tree,
    });

    let clickTimer: ReturnType<typeof setTimeout> | null = $state(null);
    let lastClickedPath = $state<string | null>(null);

    function handleFileClick(node: ProjectNode) {
        if (clickTimer && lastClickedPath === node.path) {
            // second click on same file within window - double click
            clearTimeout(clickTimer);
            clickTimer = null;
            lastClickedPath = null;
            onFileDblClick(node);
        } else {
            // first click - act immediately, set timer to catch double-click
            if (clickTimer) clearTimeout(clickTimer);
            onFileSelect(node);
            lastClickedPath = node.path;
            clickTimer = setTimeout(() => {
                clickTimer = null;
                lastClickedPath = null;
            }, 350);
        }
    }
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
        <div class="flex-1 overflow-auto py-1 overlay-scrollbar">
            {@render TreeNode({ node: rootNode, expanded, loadingDirs, onToggleDir, activeFilePath, depth: 0 })}
        </div>
    {/if}
</div>

{#snippet TreeNode(props: { node: ProjectNode; expanded: Set<string>; loadingDirs: Set<string>; onToggleDir: (path: string) => void; activeFilePath: string | null; depth: number })}
    {@const { node, expanded, loadingDirs, onToggleDir, activeFilePath, depth } = props}
    {@const isOpen = expanded.has(node.path)}
    {@const isLoading = loadingDirs.has(node.path)}
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
                <FolderOpen class="h-3.5 w-3.5 me-1.5 shrink-0 fill-base-content/40"/>
            {:else}
                <Folder class="h-3.5 w-3.5 me-1.5 shrink-0 opacity-70" />
            {/if}
            <span class="whitespace-nowrap">{node.name}</span>
        </button>
        </div>
        {@const showLoader = isOpen && isLoading}

        {#if isOpen && node.children}
            {#each node.children as child (child.path)}
                {@render TreeNode({ node: child, expanded, loadingDirs, onToggleDir, activeFilePath, depth: depth + 1 })}
            {/each}
        {:else if showLoader}
            <div class="flex items-center gap-1 text-xs opacity-50 py-1" style="padding-left: {padLeft + 20}px">
                <Loader class="h-3 w-3 animate-spin" />
                <span>Loading...</span>
            </div>
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
            onclick={() => handleFileClick(node)}
        >
            <File class="h-3.5 w-3.5 me-1.5 shrink-0 fill-base-content/40" />
            <span class="whitespace-nowrap">{node.name}</span>
        </button>
        </div>
    {/if}
{/snippet}
