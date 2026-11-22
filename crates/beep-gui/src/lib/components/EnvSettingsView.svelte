<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { CheckIcon, FileCodeIcon, PencilIcon, PlusIcon, SearchIcon, Trash2Icon, XIcon } from "@lucide/svelte";
    import { environment, project } from "$lib/app-state.svelte";
    import { confirmDeleteEnv, confirmUnsavedClose } from "$lib/dialog";
    import { envNameOf } from "$lib/vars/beep-files";
    import type { Tab } from "$lib/types";
    import VarsFileTable from "$lib/components/VarsFileTable.svelte";
    import ActiveDot from "$lib/components/ActiveDot.svelte";
    import NewEnvPopup from "$lib/components/NewEnvPopup.svelte";

    interface Props {
        /** The env settings tab. Shown file = tab.filePath, edits = tab.content (tab dirty / save / close flow). */
        tab: Tab;
        /** Open an env file in its own tab. */
        onOpenFile?: (path: string) => void;
    }

    let { tab, onOpenFile }: Props = $props();

    let error = $state<string | null>(null);
    let newEnvOpen = $state(false);
    let listEl = $state<HTMLDivElement | null>(null);
    let tableEl = $state<HTMLDivElement | null>(null);
    let query = $state("");
    let renaming = $state(false);
    let renameText = $state("");
    let renameEl = $state<HTMLInputElement | null>(null);
    /** Keyboard cursor in the filtered env list (like the project tree focus ring, kept on blur). */
    let focusIdx = $state(0);

    const envDir = $derived(tab.env?.dir ?? null);
    const current = $derived(tab.env?.name ?? null);
    const path = $derived(tab.filePath ?? null);
    const dirty = $derived(path !== null && tab.content !== tab.originalContent);
    const active = $derived(environment.selected === current);
    const label = (name: string | null) => name ?? "Default";
    const shown = $derived(environment.envs.filter((e) => label(e.name).toLowerCase().includes(query.trim().toLowerCase())));
    const otherFiles = $derived(
        (environment.envs.find((e) => e.name === current)?.files ?? []).filter((f) => f !== path),
    );

    // First open: show the selected env. Later (tab switch back): keep what the tab holds.
    $effect(() => {
        if (tab.env?.loaded) {
            void run(() => environment.refresh(envDir));
            return;
        }
        void run(() => load(environment.selected));
    });

    async function run(fn: () => Promise<void>) {
        try {
            error = null;
            await fn();
        } catch (e) {
            error = String(e);
        }
    }

    /** Show env `name`: its first file in load order. Not created yet -> empty state (no file made). */
    async function load(name: string | null) {
        await environment.refresh(envDir);
        tab.env = { dir: envDir, name, loaded: true };
        tab.filePath = undefined;
        tab.content = tab.originalContent = "";
        tab.diskChanged = false;
        renaming = false;
        const i = shown.findIndex((e) => e.name === name);
        if (i >= 0) focusIdx = i;
        if (!environment.envs.find((e) => e.name === name)?.files.length) return;
        await openCurrent();
    }

    /** Read the file to edit `current` (`{}` created at the project root if none). */
    async function openCurrent() {
        const p = await environment.fileForEdit(envDir, current);
        const text = await invoke<string>("read_file_content", { path: p });
        tab.content = tab.originalContent = text;
        tab.filePath = p;
        await environment.refresh(envDir);
    }

    /** Save / drop unsaved edits first. false = user cancelled. */
    async function settle(): Promise<boolean> {
        if (!dirty) return true;
        const choice = await confirmUnsavedClose(label(current));
        if (choice === "cancel") return false;
        if (choice === "save" && path) {
            await invoke("save_file_content", { path, content: tab.content });
            tab.originalContent = tab.content;
        }
        return true;
    }

    async function pick(name: string | null) {
        if (name === current && tab.env?.loaded) return;
        await run(async () => {
            if (await settle()) await load(name);
        });
    }

    async function remove() {
        if (!path) return;
        const files = environment.envs.find((e) => e.name === current)?.files.length ?? 1;
        const choice = await confirmDeleteEnv(label(current), shortPath(path), files);
        if (choice === "cancel") return;
        await run(async () => {
            await environment.remove(envDir, current, choice === "file" ? path : null);
            await load(environment.selected);
        });
    }

    function activate(name: string | null) {
        void run(() => environment.select(name));
    }

    function startRename() {
        renameText = current ?? "";
        renaming = true;
        requestAnimationFrame(() => renameEl?.select());
    }

    /** Rename the shown env (all its files). Default -> new env, Default comes back empty. */
    async function confirmRename() {
        const to = renameText.trim();
        if (!to || to === current) {
            renaming = false;
            return;
        }
        await run(async () => {
            if (!(await settle())) return;
            await environment.rename(envDir, current, to);
            query = "";
            await load(to);
        });
    }

    /** Project tree keys: Up/Down move, Space shows the env (list keeps focus), Enter shows it and focuses the table. */
    async function onListKey(e: KeyboardEvent) {
        const n = shown.length;
        if (n === 0) return;
        if (e.key === "ArrowDown") {
            e.preventDefault();
            focusIdx = Math.min(focusIdx + 1, n - 1);
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            focusIdx = Math.max(focusIdx - 1, 0);
        } else if (e.key === " " || e.key === "Enter") {
            e.preventDefault();
            const enter = e.key === "Enter";
            await pick(shown[focusIdx]?.name ?? null);
            if (enter) requestAnimationFrame(() => tableEl?.querySelector<HTMLElement>("input, .cm-content")?.focus());
            else listEl?.focus();
        }
    }

    function shortPath(p: string): string {
        const root = project.path;
        return root && p.startsWith(root) ? p.slice(root.length).replace(/^[/\\]/, "") : p;
    }
</script>

<div class="flex h-full min-h-0">
    <!-- env list (sidebar like History / project tree) -->
    <div class="w-56 shrink-0 border-e border-base-300 flex flex-col">
        <div class="py-2 text-xs font-semibold uppercase tracking-wide select-none border-b border-b-base-content/10 flex items-center justify-between h-8">
            <span class="px-2 opacity-70">Environments</span>
            <button
                class="btn btn-ghost btn-xs btn-square me-1 tooltip tooltip-top tooltip-end"
                aria-label="New environment"
                onclick={() => (newEnvOpen = true)}
            >
                <PlusIcon class="h-3.5 w-3.5" />
                <div class="tooltip-content text-xs">New environment</div>
            </button>
        </div>
        <label class="flex items-center gap-1 h-7 px-2 border-b border-b-base-content/10 text-xs">
            <SearchIcon class="h-3 w-3 opacity-50 shrink-0" />
            <input
                class="flex-1 min-w-0 bg-transparent outline-none"
                placeholder="Search environments"
                aria-label="Search environments"
                bind:value={query}
                oninput={() => (focusIdx = 0)}
                onkeydown={(e) => {
                    if (e.key === "Escape") query = "";
                    else if (e.key === "ArrowDown") {
                        e.preventDefault();
                        listEl?.focus();
                    }
                }}
            />
            {#if query}
                <button class="btn btn-ghost btn-xs btn-square h-5 w-5 min-h-0" aria-label="Clear search" onclick={() => (query = "")}>
                    <XIcon class="h-3 w-3" />
                </button>
            {/if}
        </label>
        <div
            bind:this={listEl}
            class="flex-1 overflow-y-auto py-1 outline-none"
            role="listbox"
            aria-label="Environments"
            tabindex="0"
            onkeydown={onListKey}
        >
            {#each shown as env, i (env.name ?? "")}
                <!-- keys live on the listbox -->
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <div
                    role="option"
                    tabindex="-1"
                    aria-selected={env.name === current}
                    class="group/active flex items-center w-full text-left text-xs h-6 px-2 cursor-pointer hover:bg-base-200/50
                        {env.name === current ? 'bg-base-200' : ''} {i === focusIdx ? 'ring-1 ring-inset ring-primary/50' : ''}"
                    onclick={() => {
                        focusIdx = i;
                        listEl?.focus();
                        void pick(env.name);
                    }}
                >
                    <span class="truncate">{label(env.name)}</span>
                    {#if environment.selected === env.name}
                        <ActiveDot class="ms-1.5" />
                    {/if}
                    {#if env.files.length === 0}
                        <span class="ms-2 opacity-40 italic whitespace-nowrap">not created</span>
                    {/if}
                    {#if environment.selected !== env.name}
                        <button
                            class="ms-auto btn btn-ghost btn-xs h-5 min-h-0 px-1 gap-1 font-normal opacity-60 hover:opacity-100 group-hover/active:flex
                                {i === focusIdx ? 'flex' : 'hidden'}"
                            tabindex="-1"
                            onclick={(e) => {
                                e.stopPropagation();
                                activate(env.name);
                            }}
                        >
                            <CheckIcon class="h-3 w-3" />
                            Activate
                        </button>
                    {/if}
                </div>
            {:else}
                <div class="px-2 py-1 text-xs opacity-40">No match</div>
            {/each}
        </div>
    </div>

    <!-- selected env -->
    <div class="flex-1 min-w-0 flex flex-col">
        <!-- toolbar: same height / look as MainEditorToolbar -->
        <div class="flex items-center gap-2 h-8 border-b border-base-content/10 text-xs shrink-0 select-none ps-2">
            {#if renaming}
                <input
                    bind:this={renameEl}
                    bind:value={renameText}
                    class="input input-xs w-48"
                    aria-label="Environment name"
                    onkeydown={(e) => {
                        if (e.key === "Enter") void confirmRename();
                        else if (e.key === "Escape") renaming = false;
                    }}
                />
                <button class="btn btn-ghost btn-xs btn-square text-success tooltip tooltip-top tooltip-end" aria-label="Rename" onclick={confirmRename}>
                    <CheckIcon class="h-3.5 w-3.5" />
                    <div class="tooltip-content text-xs">Rename</div>
                </button>
                <button class="btn btn-ghost btn-xs btn-square tooltip tooltip-top tooltip-end" aria-label="Cancel" onclick={() => (renaming = false)}>
                    <XIcon class="h-3.5 w-3.5" />
                    <div class="tooltip-content text-xs">Cancel</div>
                </button>
            {:else}
                <span class="shrink-0 font-mono">{label(current)}</span>
                <span class="opacity-30 font-mono">|</span>
                <span class="opacity-60 font-mono truncate">{path ? shortPath(path) : "not created"}</span>
                {#if active}
                    <span class="badge badge-xs badge-primary gap-0.5 shrink-0"><CheckIcon class="h-2.5 w-2.5" />Active</span>
                {:else}
                    <button class="btn btn-ghost btn-xs gap-1 font-normal shrink-0" onclick={() => activate(current)}>
                        <CheckIcon class="h-3.5 w-3.5" />
                        Activate Environment
                    </button>
                {/if}
            {/if}
            {#if error}
                <span class="text-error truncate">{error}</span>
            {/if}
            <div class="flex-1"></div>
            {#if path}
                {#if !renaming}
                    <button class="btn btn-xs btn-ghost btn-square tooltip tooltip-top tooltip-end" aria-label="Rename environment" onclick={startRename}>
                        <PencilIcon class="h-3.5 w-3.5" />
                        <div class="tooltip-content text-xs">Rename environment</div>
                    </button>
                {/if}
                <div class="divider divider-horizontal w-1 m-0"></div>
                <div class="flex items-center gap-1 pe-1 shrink-0">
                    <button class="btn btn-xs btn-ghost btn-square tooltip tooltip-top tooltip-end" aria-label="Open file" onclick={() => path && onOpenFile?.(path)}>
                        <FileCodeIcon class="h-3.5 w-3.5" />
                        <div class="tooltip-content text-xs">Open file</div>
                    </button>
                    <button class="btn btn-xs btn-ghost btn-square text-error tooltip tooltip-top tooltip-end" aria-label="Delete environment" onclick={remove}>
                        <Trash2Icon class="h-3.5 w-3.5" />
                        <div class="tooltip-content text-xs">Delete environment</div>
                    </button>
                </div>
            {/if}
        </div>
        <div class="flex-1 min-h-0 overflow-y-auto" bind:this={tableEl}>
            {#if path}
                <VarsFileTable content={tab.content} onchange={(v) => (tab.content = v)} onShowCode={() => path && onOpenFile?.(path)} />
            {:else}
                <div class="p-6 flex flex-col items-center gap-2 text-sm opacity-70">
                    <span>"{label(current)}" has no file yet.</span>
                    <button class="btn btn-xs btn-primary" onclick={() => run(openCurrent)}>Create file</button>
                </div>
            {/if}
        </div>
        {#if otherFiles.length > 0}
            <div class="px-2 py-2 border-t border-base-content/10 text-xs flex flex-col gap-0.5">
                <span class="opacity-60">Also set in</span>
                {#each otherFiles as f (f)}
                    <button class="text-left font-mono link link-hover truncate opacity-70" onclick={() => onOpenFile?.(f)}>{shortPath(f)}</button>
                {/each}
            </div>
        {/if}
    </div>
</div>

<NewEnvPopup
    open={newEnvOpen}
    onclose={() => (newEnvOpen = false)}
    oncreated={(p) => void pick(envNameOf(p))}
/>
