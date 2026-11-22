<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { FileCodeIcon, PlusIcon, Trash2Icon } from "@lucide/svelte";
    import { environment, project } from "$lib/app-state.svelte";
    import { varIndex } from "$lib/vars/var-index.svelte";
    import { confirmDeleteEnv, confirmUnsavedClose } from "$lib/dialog";
    import VarsFileTable from "$lib/components/VarsFileTable.svelte";

    interface Props {
        open: boolean;
        /** Env files are looked up from this dir. */
        envDir: string | null;
        onclose: () => void;
        /** Open an env file in a tab. */
        onOpenFile?: (path: string) => void;
    }

    let { open, envDir, onclose, onOpenFile }: Props = $props();

    let dialog = $state<HTMLDialogElement | null>(null);
    /** Env shown on the right (null = Default). */
    let current = $state<string | null>(null);
    let path = $state<string | null>(null);
    let content = $state("");
    let original = $state("");
    let error = $state<string | null>(null);
    let newName = $state("");

    const dirty = $derived(path !== null && content !== original);
    const label = (name: string | null) => name ?? "Default";
    const otherFiles = $derived(
        (environment.envs.find((e) => e.name === current)?.files ?? []).filter((f) => f !== path),
    );

    $effect(() => {
        if (!dialog) return;
        if (open && !dialog.open) {
            dialog.showModal();
            void start();
        } else if (!open && dialog.open) {
            dialog.close();
        }
    });

    async function start() {
        error = null;
        newName = "";
        await run(async () => {
            await environment.refresh(envDir);
            await load(environment.selected);
        });
    }

    async function run(fn: () => Promise<void>) {
        try {
            await fn();
        } catch (e) {
            error = String(e);
        }
    }

    /** Show env `name`: its first file in load order. Not created yet -> empty state (no file made). */
    async function load(name: string | null) {
        await environment.refresh(envDir);
        current = name;
        path = null;
        content = original = "";
        if (!environment.envs.some((e) => e.name === name && e.files.length > 0)) return;
        await openCurrent();
    }

    /** Read the file to edit `current` (`{}` created at the project root if none). */
    async function openCurrent() {
        const p = await environment.fileForEdit(envDir, current);
        content = original = await invoke<string>("read_file_content", { path: p });
        path = p;
        await environment.refresh(envDir);
    }

    /** Save / drop unsaved edits first. false = user cancelled. */
    async function settle(): Promise<boolean> {
        if (!dirty) return true;
        const choice = await confirmUnsavedClose(label(current));
        if (choice === "cancel") return false;
        if (choice === "save") await save();
        return true;
    }

    async function save() {
        if (!path) return;
        await invoke("save_file_content", { path, content });
        original = content;
        await varIndex.refresh();
    }

    async function pick(name: string | null) {
        if (name === current || !(await settle())) return;
        await run(() => load(name));
    }

    async function create() {
        const name = newName.trim();
        if (!name || !(await settle())) return;
        await run(async () => {
            await environment.create(name);
            newName = "";
            await load(name);
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

    async function openFile(p: string) {
        if (!(await settle())) return;
        onOpenFile?.(p);
        onclose();
    }

    async function close() {
        if (await settle()) onclose();
    }

    function shortPath(p: string): string {
        const root = project.path;
        return root && p.startsWith(root) ? p.slice(root.length).replace(/^[/\\]/, "") : p;
    }
</script>

<!-- Esc fires `cancel`: handle it like the Cancel button (asks when dirty). -->
<dialog
    class="modal"
    bind:this={dialog}
    oncancel={(e) => {
        e.preventDefault();
        void close();
    }}
>
    <div class="modal-box border border-base-content/10 max-w-4xl w-full h-[70vh] p-0 flex flex-col">
        <div class="flex flex-1 min-h-0">
            <!-- env list -->
            <div class="w-48 shrink-0 border-e border-base-content/10 flex flex-col">
                <div class="px-3 py-2 text-xs uppercase tracking-wider opacity-60">Environments</div>
                <ul class="menu menu-xs w-full flex-1 overflow-y-auto flex-nowrap [&>li]:shrink-0">
                    {#each environment.envs as env (env.name ?? "")}
                        <li>
                            <button class:menu-active={env.name === current} onclick={() => pick(env.name)}>
                                <span class="truncate flex-1">{label(env.name)}</span>
                                {#if environment.selected === env.name}
                                    <span class="badge badge-primary badge-xs">active</span>
                                {:else if env.files.length === 0}
                                    <span class="opacity-40 italic">not created</span>
                                {/if}
                            </button>
                        </li>
                    {/each}
                </ul>
                <form
                    class="p-2 border-t border-base-content/10 flex gap-1"
                    onsubmit={(e) => {
                        e.preventDefault();
                        void create();
                    }}
                >
                    <input class="input input-xs flex-1 min-w-0" placeholder="New environment" bind:value={newName} spellcheck="false" />
                    <button type="submit" class="btn btn-xs btn-square tooltip tooltip-top" aria-label="Create environment" disabled={!newName.trim()}>
                        <PlusIcon class="h-3 w-3" />
                        <div class="tooltip-content text-xs">Create environment</div>
                    </button>
                </form>
            </div>

            <!-- selected env -->
            <div class="flex-1 min-w-0 flex flex-col">
                <div class="flex items-center gap-2 px-3 py-2 border-b border-base-content/10">
                    <span class="font-semibold truncate">{label(current)}</span>
                    {#if environment.selected === current}
                        <span class="badge badge-primary badge-sm">Active</span>
                    {:else}
                        <button class="btn btn-xs btn-ghost" onclick={() => run(() => environment.select(current))}>
                            Use this environment
                        </button>
                    {/if}
                    <div class="ms-auto flex items-center gap-1">
                        {#if path}
                            <button class="btn btn-xs btn-ghost btn-square tooltip tooltip-top" aria-label="Open file" onclick={() => path && openFile(path)}>
                                <FileCodeIcon class="h-3.5 w-3.5" />
                                <div class="tooltip-content text-xs">Open file ({shortPath(path)})</div>
                            </button>
                            <button class="btn btn-xs btn-ghost btn-square text-error tooltip tooltip-end" aria-label="Delete environment" onclick={remove}>
                                <Trash2Icon class="h-3.5 w-3.5" />
                                <div class="tooltip-content text-xs">Delete environment</div>
                            </button>
                        {/if}
                    </div>
                </div>
                <div class="flex-1 min-h-0 overflow-auto">
                    {#if path}
                        <VarsFileTable {content} onchange={(v) => (content = v)} onShowCode={() => path && openFile(path)} />
                    {:else}
                        <div class="p-6 flex flex-col items-center gap-2 text-sm opacity-70">
                            <span>"{label(current)}" has no file yet.</span>
                            <button class="btn btn-xs" onclick={() => run(openCurrent)}>Create file</button>
                        </div>
                    {/if}
                </div>
                {#if otherFiles.length > 0}
                    <div class="px-3 py-2 border-t border-base-content/10 text-xs flex flex-col gap-0.5">
                        <span class="opacity-60">Also set in</span>
                        {#each otherFiles as f (f)}
                            <button class="text-left link link-hover truncate opacity-70" onclick={() => openFile(f)}>{shortPath(f)}</button>
                        {/each}
                    </div>
                {/if}
            </div>
        </div>

        <div class="flex items-center gap-2 px-3 py-2 border-t border-base-content/10">
            {#if error}
                <span class="text-error text-xs truncate">{error}</span>
            {/if}
            <div class="ms-auto flex gap-2">
                <button class="btn btn-sm btn-ghost" onclick={close}>Close</button>
                <button class="btn btn-sm btn-primary" disabled={!dirty} onclick={() => run(save)}>Save</button>
            </div>
        </div>
    </div>
</dialog>
