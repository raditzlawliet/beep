<script lang="ts">
    import type { EditorLanguage, HttpResponse } from "$lib/types";
    import { LANGUAGE_LABELS } from "$lib/language-sniff";
    import { HistoryIcon, FolderTree, PlusIcon } from "@lucide/svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import HorizontalDivider from "./uis/HorizontalDivider.svelte";
    import { app, environment, project } from "$lib/app-state.svelte";
    import NewEnvPopup from "$lib/components/NewEnvPopup.svelte";
    import ActiveDot from "$lib/components/ActiveDot.svelte";
    import { varIndex } from "$lib/vars/var-index.svelte";
    import { TriangleAlertIcon } from "@lucide/svelte";
    import { editorCursor } from "$lib/editor-cursor.svelte";

    interface Props {
        onToggleHistoryFocus: () => void;
        onToggleProjectFocus: () => void;
        activePanel: "history" | "project";
        sidebarOpen: boolean;
        hasProject: boolean;
        response: HttpResponse | null;
        loading: boolean;
        error: string | null;
        /** Active file tab language; null hides the picker. */
        language?: EditorLanguage | null;
        onLanguageSelect?: (lang: EditorLanguage) => void;
        /** Show the environment picker (project open); env files are looked up from this dir. */
        showEnv?: boolean;
        envDir?: string | null;
        /** Open an env file in a tab (created / picked from the env picker). */
        onOpenEnvFile?: (path: string) => void;
    }

    let {
        onToggleHistoryFocus,
        onToggleProjectFocus,
        activePanel,
        sidebarOpen,
        hasProject,
        response,
        loading,
        error,
        language = null,
        onLanguageSelect,
        showEnv = false,
        envDir = null,
        onOpenEnvFile,
    }: Props = $props();

    let newEnvOpen = $state(false);
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

    const LANGUAGES = Object.keys(LANGUAGE_LABELS) as EditorLanguage[];

    function selectEnv(name: string | null) {
        environment.select(name).catch((e) => console.error("select env", e));
        closeMenu();
    }

    function selectLanguage(lang: EditorLanguage) {
        onLanguageSelect?.(lang);
        // Close the focus-driven dropdown.
        (document.activeElement as HTMLElement | null)?.blur();
    }
</script>

<div
    class="flex items-center h-8 mx-1 border-t border-t-base-content/10 gap-1 text-xs"
>
    <div class="tooltip tooltip-start">
        <button
            class="btn btn-ghost btn-xs btn-square"
            onclick={onToggleHistoryFocus}
            aria-label="Toggle History Panel"
        >
            <span class:text-primary={sidebarOpen && activePanel === "history"}>
                <HistoryIcon class="h-3 w-3" />
            </span>
        </button>
        <div class="tooltip-content text-xs">
            <span>History Panel</span>
            <span class="opacity-50 ml-4">{app.modKey}+Shift+H</span>
        </div>
    </div>
    <div class="tooltip tooltip-start">
        <button
            class="btn btn-ghost btn-xs btn-square"
            onclick={onToggleProjectFocus}
            aria-label="Toggle Project Panel"
        >
            <span class:text-primary={sidebarOpen && activePanel === "project"}>
                <FolderTree class="h-3 w-3" />
            </span>
        </button>
        <div class="tooltip-content text-xs">
            <span>Project Panel</span>
            <span class="opacity-50 ml-4">{app.modKey}+Shift+E</span>
        </div>
    </div>
    <div class="divider divider-horizontal w-1 m-0"></div>

    {#if loading}
        <span class="text-xs opacity-70">Loading...</span>
    {:else if error}
        <span class="text-xs text-error">{error}</span>
    {:else if response}
        <span class="flex items-center gap-1.5">
            <StatusBadge status={response.status} />
            <span class="opacity-30">·</span>
            <span class="opacity-70">{response.elapsed_ms}ms</span>
        </span>
    {:else}
        <span class="text-xs opacity-50">Ready</span>
    {/if}
    <div class="flex-1"></div>
    {#if varIndex.unknown.length > 0}
        {@const unknown = varIndex.unknown}
        <div class="tooltip tooltip-top">
            <span class="flex items-center gap-1 text-warning px-1">
                <TriangleAlertIcon class="h-3 w-3" />
                {unknown.length} unknown {unknown.length === 1 ? "var" : "vars"}
            </span>
            <div class="tooltip-content text-xs text-left">
                <div class="opacity-70 mb-1">Sent as text:</div>
                {#each unknown as key (key)}
                    <div class="font-mono">{`{{${key}}}`}</div>
                {/each}
            </div>
        </div>
    {/if}
    {#if editorCursor.current}
        {@const cur = editorCursor.current}
        <div class="tooltip tooltip-top">
            <button
                class="btn btn-ghost btn-xs font-normal font-mono opacity-70 hover:opacity-100"
                onclick={() => editorCursor.openGoto()}
                aria-label="Go to Line/Column"
            >
                {cur.line}:{cur.col}
            </button>
            <div class="tooltip-content text-xs">
                <span>Go to Line/Column</span>
                <span class="opacity-50 ml-4">{app.modKey}+G</span>
            </div>
        </div>
    {/if}
    {#if language}
        <div class="dropdown dropdown-top dropdown-end">
            <div
                tabindex="0"
                role="button"
                class="btn btn-ghost btn-xs font-normal opacity-70 hover:opacity-100"
                aria-label="Select Language"
            >
                {LANGUAGE_LABELS[language]}
            </div>
            <ul
                tabindex="-1"
                class="dropdown-content menu menu-xs bg-base-200 border border-base-content/10 rounded-box z-50 w-40 p-1 shadow-lg mb-1"
            >
                {#each LANGUAGES as lang (lang)}
                    <li>
                        <button
                            class:menu-active={lang === language}
                            onclick={() => selectLanguage(lang)}
                        >
                            {LANGUAGE_LABELS[lang]}
                        </button>
                    </li>
                {/each}
            </ul>
        </div>
    {/if}
    {#if showEnv}
        <div class="divider divider-horizontal w-1 m-0"></div>
        <div class="dropdown dropdown-top dropdown-end">
            <div
                tabindex="0"
                role="button"
                class="btn btn-ghost btn-xs font-normal opacity-70 hover:opacity-100 tooltip tooltip-end"
                aria-label="Select Environment"
                onfocus={() => environment.refresh(envDir).catch(() => {})}
            >
                {environment.selected ?? "Default"}
                <div class="tooltip-content text-xs">Environment</div>
            </div>
            <ul
                tabindex="-1"
                class="dropdown-content menu menu-xs bg-base-200 border border-base-content/10 rounded-box z-50 w-64 p-1 shadow-lg mb-1 flex-nowrap [&>li]:shrink-0"
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
            </ul>
        </div>
        <NewEnvPopup open={newEnvOpen} onclose={() => (newEnvOpen = false)} oncreated={(path) => onOpenEnvFile?.(path)} />
    {/if}
</div>
