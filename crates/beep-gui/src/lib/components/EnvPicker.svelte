<script lang="ts">
    import { LayersIcon, PlusIcon, Settings2Icon } from "@lucide/svelte";
    import { environment, project } from "$lib/app-state.svelte";
    import NewEnvPopup from "$lib/components/NewEnvPopup.svelte";
    import ActiveDot from "$lib/components/ActiveDot.svelte";
    import EnvSettingsDialog from "$lib/components/modals/EnvSettingsDialog.svelte";

    interface Props {
        /** Env files are looked up from this dir. */
        envDir?: string | null;
        /** Open an env file in a tab (created / picked from the env picker). */
        onOpenEnvFile?: (path: string) => void;
    }

    let { envDir = null, onOpenEnvFile }: Props = $props();

    let newEnvOpen = $state(false);
    let settingsOpen = $state(false);
    let envError = $state<string | null>(null);

    const missingSelected = $derived(
        environment.selected !== null && !environment.envs.some((e) => e.name === environment.selected),
    );

    function closeMenu() {
        (document.activeElement as HTMLElement | null)?.blur();
    }

    async function openEnv(name: string | null) {
        try {
            const path = await environment.fileForEdit(envDir, name);
            envError = null;
            closeMenu();
            onOpenEnvFile?.(path);
        } catch (e) {
            envError = String(e);
        }
    }

    function openFile(path: string) {
        closeMenu();
        onOpenEnvFile?.(path);
    }

    /** Path shown relative to the project root. */
    function shortPath(path: string): string {
        const root = project.path;
        return root && path.startsWith(root) ? path.slice(root.length).replace(/^[/\\]/, "") : path;
    }

    function selectEnv(name: string | null) {
        environment.select(name).catch((e) => console.error("select env", e));
        closeMenu();
    }
</script>

<div class="dropdown dropdown-bottom dropdown-start">
    <div
        tabindex="0"
        role="button"
        class="btn btn-ghost btn-xs font-normal gap-1 ms-1 opacity-70 hover:opacity-100 tooltip tooltip-bottom"
        aria-label="Select Environment"
        onfocus={() => environment.refresh(envDir).catch(() => {})}
    >
        <LayersIcon class="h-3 w-3" />
        {environment.selected ?? "Default"}
        <div class="tooltip-content text-xs">Environment</div>
    </div>
    <ul
        tabindex="-1"
        class="dropdown-content menu menu-xs bg-base-200 border border-base-content/10 rounded-box z-50 w-64 p-1 shadow-lg mt-1 flex-nowrap [&>li]:shrink-0"
    >
        <li class="menu-title flex-row items-center justify-between py-1 pe-1 uppercase tracking-wider">
            <span>Environments</span>
            <button
                class="btn btn-ghost btn-xs btn-square h-5 w-5 min-h-0 tooltip tooltip-end normal-case tracking-normal font-normal"
                aria-label="New environment"
                onclick={() => {
                    closeMenu();
                    newEnvOpen = true;
                }}
            >
                <PlusIcon class="h-3 w-3" />
                <div class="tooltip-content text-xs">New environment</div>
            </button>
        </li>
        {#each environment.envs as env (env.name ?? "")}
            <li class="group group/active">
                <div class="flex items-center gap-2">
                    <button class="flex-1 min-w-0 flex items-center gap-1.5 text-left" onclick={() => selectEnv(env.name)}>
                        <span class="truncate">{env.name ?? "Default"}</span>
                        {#if environment.selected === env.name}
                            <ActiveDot />
                        {/if}
                        {#if env.files.length === 0}
                            <span class="opacity-40 italic whitespace-nowrap">not created yet</span>
                        {/if}
                    </button>
                    <button
                        class="opacity-0 group-hover:opacity-60 hover:opacity-100! hover:underline underline-offset-2 tooltip tooltip-end"
                        onclick={() => openEnv(env.name)}
                    >
                        Edit
                        <div class="tooltip-content text-xs">{env.files.length ? "Open env file" : "Create and open env file"}</div>
                    </button>
                </div>
            </li>
            <!-- load order: first = project root, last wins -->
            {#each env.files as file, i (file)}
                <li>
                    <button class="ps-6 opacity-60 hover:opacity-100 tooltip tooltip-end" onclick={() => openFile(file)}>
                        <span class="opacity-50">{i + 1}.</span>
                        <span class="truncate">{shortPath(file)}</span>
                        <div class="tooltip-content text-xs break-all">{file}</div>
                    </button>
                </li>
            {/each}
        {/each}
        {#if missingSelected}
            <li class="disabled">
                <span>{environment.selected} <span class="italic">file missing</span></span>
            </li>
        {/if}
        {#if envError}
            <li class="text-error px-2 whitespace-normal">{envError}</li>
        {/if}
        <li class="border-t border-base-content/10 mt-1 pt-1">
            <button
                onclick={() => {
                    closeMenu();
                    settingsOpen = true;
                }}
            >
                <Settings2Icon class="h-3 w-3" />
                Manage environments...
            </button>
        </li>
    </ul>
</div>
<EnvSettingsDialog open={settingsOpen} {envDir} onclose={() => (settingsOpen = false)} onOpenFile={(path) => onOpenEnvFile?.(path)} />
<NewEnvPopup open={newEnvOpen} onclose={() => (newEnvOpen = false)} oncreated={(path) => onOpenEnvFile?.(path)} />
