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

    // -- Flat visible nodes for keyboard navigation (Zed/VSCode style)

    interface FlatNode {
        node: ProjectNode;
        depth: number;
    }

    let flatNodes = $derived.by<FlatNode[]>(() => {
        const result: FlatNode[] = [];
        function walk(n: ProjectNode, depth: number) {
            result.push({ node: n, depth });
            if (n.is_dir && expanded.has(n.path) && n.children) {
                for (const child of n.children) {
                    walk(child, depth + 1);
                }
            }
        }
        walk(rootNode, 0);
        return result;
    });

    let focusedPath = $state<string | null>(null);
    let pendingExpandPath = $state<string | null>(null);
    let containerEl = $state<HTMLDivElement | null>(null);

    // Auto-focus first child after keyboard-triggered expand finishes loading
    $effect(() => {
        const pending = pendingExpandPath;
        if (!pending) return;
        // Not loading anymore; either children arrived or dir is genuinely empty
        if (!loadingDirs.has(pending)) {
            pendingExpandPath = null;
            const fn = flatNodes.find(f => f.node.path === pending);
            if (fn && fn.node.children && fn.node.children.length > 0) {
                focusedPath = fn.node.children[0].path;
            }
        }
    });

    // Sync focusedPath to activeFilePath when tab selection changes
    $effect(() => {
        if (activeFilePath) {
            focusedPath = activeFilePath;
        }
    });

    // Auto-scroll to active file when it changes
    $effect(() => {
        const path = activeFilePath;
        if (path && containerEl) {
            requestAnimationFrame(() => {
                const el = containerEl!.querySelector(`[data-path="${CSS.escape(path)}"]`);
                if (el) el.scrollIntoView({ block: "nearest", behavior: "instant" });
            });
        }
    });

    // Auto-scroll to focused node during keyboard navigation
    $effect(() => {
        const path = focusedPath;
        if (path && containerEl) {
            requestAnimationFrame(() => {
                const el = containerEl!.querySelector(`[data-path="${CSS.escape(path)}"]`);
                if (el) el.scrollIntoView({ block: "nearest", behavior: "instant" });
            });
        }
    });

    function findParentNode(childPath: string): ProjectNode | undefined {
        function search(n: ProjectNode): ProjectNode | undefined {
            if (n.children) {
                for (const child of n.children) {
                    if (child.path === childPath) return n;
                    const found = search(child);
                    if (found) return found;
                }
            }
            return undefined;
        }
        return search(rootNode);
    }

    function focusFirstVisible() {
        if (flatNodes.length > 0) {
            focusedPath = flatNodes[0].node.path;
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        const idx = flatNodes.findIndex(fn => fn.node.path === focusedPath);

        if (idx === -1) {
            focusFirstVisible();
            return;
        }

        const flat = flatNodes[idx];
        const node = flat.node;

        // -- Navigation
        if (e.key === "ArrowDown") {
            e.preventDefault();
            if (idx < flatNodes.length - 1) {
                focusedPath = flatNodes[idx + 1].node.path;
            }
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            if (idx > 0) {
                focusedPath = flatNodes[idx - 1].node.path;
            }
        } else if (e.key === "ArrowRight") {
            // -- Expand / go to first child
            e.preventDefault();
            if (node.is_dir) {
                if (!expanded.has(node.path)) {
                    if (!node.children || node.children.length === 0) {
                        pendingExpandPath = node.path;
                    }
                    onToggleDir(node.path);
                } else if (node.children && node.children.length > 0) {
                    focusedPath = node.children[0].path;
                }
            }
        } else if (e.key === "ArrowLeft") {
            // -- Collapse / go to parent
            e.preventDefault();
            if (node.is_dir && expanded.has(node.path)) {
                onToggleDir(node.path);
                // Focus stays on this dir
            } else {
                const parent = findParentNode(node.path);
                if (parent) {
                    focusedPath = parent.path;
                }
            }
        } else if (e.key === " ") {
            // -- Space: toggle dir / open file (temp, keep tree focus)
            e.preventDefault();
            if (node.is_dir) {
                onToggleDir(node.path);
            } else {
                onFileSelect(node);
                requestAnimationFrame(() => containerEl?.focus());
            }
        } else if (e.key === "Enter") {
            // -- Enter: toggle dir / open file (persistent, focus editor)
            e.preventDefault();
            if (node.is_dir) {
                onToggleDir(node.path);
            } else {
                onFileDblClick(node);
            }
        }
    }

    function handleDirClick(node: ProjectNode) {
        focusedPath = node.path;
        onToggleDir(node.path);
    }

    function handleFileClick(node: ProjectNode) {
        focusedPath = node.path;
        if (clickTimer && lastClickedPath === node.path) {
            // second click on same file within window - double click (persistent, focus editor)
            clearTimeout(clickTimer);
            clickTimer = null;
            lastClickedPath = null;
            onFileDblClick(node);
        } else {
            // first click - preview file but keep tree focused
            if (clickTimer) clearTimeout(clickTimer);
            onFileSelect(node);
            lastClickedPath = node.path;
            requestAnimationFrame(() => containerEl?.focus());
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
        <div
            class="flex-1 overflow-auto py-1 overlay-scrollbar outline-none"
            bind:this={containerEl}
            tabindex="-1"
            role="tree"
            data-panel="project"
            onkeydown={handleKeydown}
            onfocus={() => {
                if (!focusedPath) focusFirstVisible();
            }}
        >
            {@render TreeNode({ node: rootNode, expanded, loadingDirs, onToggleDir, activeFilePath, focusedPath, depth: 0 })}
        </div>
    {/if}
</div>

{#snippet GuideLines(depth: number)}
    {#if depth > 0}
        {#each Array(depth) as _, d (d)}
            <div
                class="absolute top-0 bottom-0 w-px bg-base-content/10 pointer-events-none"
                style="left: {d * 20 + 14}px"
            ></div>
        {/each}
    {/if}
{/snippet}

{#snippet TreeNode(props: { node: ProjectNode; expanded: Set<string>; loadingDirs: Set<string>; onToggleDir: (path: string) => void; activeFilePath: string | null; focusedPath: string | null; depth: number })}
    {@const { node, expanded, loadingDirs, onToggleDir, activeFilePath, focusedPath, depth } = props}
    {@const isOpen = expanded.has(node.path)}
    {@const isLoading = loadingDirs.has(node.path)}
    {@const isActive = activeFilePath === node.path}
    {@const isFocused = focusedPath === node.path}
    {@const padLeft = depth * 20 + 8}

    {#if node.is_dir}
        <div class="relative">
            {@render GuideLines(depth)}
        <button
            data-path={node.path}
            class="flex items-center w-full text-left text-xs py-1 hover:bg-base-200/50 {isFocused ? 'ring-1 ring-inset ring-primary/50' : ''}"
            style="padding-left: {padLeft}px"
            onclick={() => handleDirClick(node)}
        >
            {#if isOpen}
                <FolderOpen class="h-3.5 w-3.5 me-1.5 shrink-0 fill-base-content/40"/>
            {:else}
                <Folder class="h-3.5 w-3.5 me-1.5 shrink-0 opacity-70" />
            {/if}
            <span class="whitespace-nowrap">{node.name}</span>
        </button>
        </div>
        {#if isOpen && isLoading}
            <div class="flex items-center gap-1 text-xs opacity-50 py-1" style="padding-left: {padLeft + 20}px">
                <Loader class="h-3 w-3 animate-spin" />
                <span>Loading...</span>
            </div>
        {:else if isOpen && node.children}
            {#each node.children as child (child.path)}
                {@render TreeNode({ node: child, expanded, loadingDirs, onToggleDir, activeFilePath, focusedPath, depth: depth + 1 })}
            {/each}
        {/if}
    {:else}
        <div class="relative">
            {@render GuideLines(depth)}
        <button
            data-path={node.path}
            class="flex items-center w-full text-left text-xs py-1 hover:bg-base-200/50 {isActive ? 'bg-base-200' : ''} {isFocused ? 'ring-1 ring-inset ring-primary/50' : ''}"
            style="padding-left: {padLeft}px"
            onclick={() => handleFileClick(node)}
        >
            <File class="h-3.5 w-3.5 me-1.5 shrink-0 fill-base-content/40" />
            <span class="whitespace-nowrap">{node.name}</span>
        </button>
        </div>
    {/if}
{/snippet}
