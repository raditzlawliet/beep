<script lang="ts">
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { X, Minus, Square, MenuIcon } from "@lucide/svelte";
    import Unmaximize from "$lib/components/icons/Unmaximize.svelte";
    import AboutDialog from "$lib/components/modals/AboutDialog.svelte";

    interface Props {
        onNewRequest: () => void;
        onOpenProject: () => void;
        onCloseProject: () => void;
        projectName: string | null;
    }

    let { onNewRequest, onOpenProject, onCloseProject, projectName }: Props = $props();

    let aboutDialog = $state<HTMLDialogElement | null>(null);

    let menuOpen = $state(false);
    let maximized = $state(false);

    function toggleMenu(e: MouseEvent) {
        menuOpen = !menuOpen;
        e.stopPropagation();
    }

    function closeAll() {
        menuOpen = false;
    }

    function handleNewRequest() {
        onNewRequest();
        closeAll();
    }

    function handleOpenProject() {
        onOpenProject();
        closeAll();
    }

    function handleCloseProject() {
        onCloseProject();
        closeAll();
    }

    function handleAbout() {
        aboutDialog?.showModal();
        closeAll();
    }

    async function quit() {
        await getCurrentWindow().close();
    }

    async function minimize() {
        await getCurrentWindow().minimize();
    }

    async function toggleMaximize() {
        const win = getCurrentWindow();
        if (await win.isMaximized()) {
            await win.unmaximize();
            maximized = false;
        } else {
            await win.maximize();
            maximized = true;
        }
    }

    async function closeWindow() {
        await getCurrentWindow().close();
    }

    function handleKeydown(e: KeyboardEvent) {}

    function handleWindowClick() {
        if (menuOpen) closeAll();
    }

    // auto-detect maximize state on window resize / drag
    $effect(() => {
        const win = getCurrentWindow();
        win.isMaximized().then((m) => (maximized = m));
        const onResize = () => {
            win.isMaximized().then((m) => (maximized = m));
        };
        window.addEventListener("resize", onResize);
        return () => window.removeEventListener("resize", onResize);
    });
</script>

<svelte:window onkeydown={handleKeydown} onclick={handleWindowClick} />

<div
    class="flex items-center h-8 select-none border-b border-b-base-content/10"
    role="presentation"
>
    <!-- left: hamburger / menu bar -->
    {#if menuOpen}
        <div
            class="flex items-center h-full ms-1 relative z-30"
            role="menubar"
            tabindex="-1"
        >
            <!-- Beep Dropdown -->
            <div class="dropdown dropdown-hover dropdown-start h-full">
                <button
                    class="btn btn-ghost btn-xs rounded-none h-full text-xs font-medium"
                    role="menu"
                    tabindex="0"
                >
                    Beep
                </button>
                <ul
                    class="dropdown-content menu menu-sm bg-base-200 rounded-box z-1 shadow-sm border border-base-content/10 w-52 p-1"
                    tabindex="-1"
                >
                    <li><button onclick={handleAbout}>About Beep</button></li>
                    <li></li>
                    <li><button onclick={quit}>Quit</button></li>
                </ul>
            </div>

            <!-- File Dropdown -->
            <div class="dropdown dropdown-hover dropdown-start h-full">
                <button
                    class="btn btn-ghost btn-xs rounded-none h-full text-xs font-medium"
                    role="menu"
                    tabindex="0"
                >
                    File
                </button>
                <ul
                    class="dropdown-content menu menu-sm bg-base-200 rounded-box z-1 shadow-sm border border-base-content/10 w-52 p-1"
                    tabindex="-1"
                >
                    <li>
                        <button onclick={handleNewRequest}>New Request</button>
                    </li>
                    <li></li>
                    <li>
                        <button onclick={handleOpenProject}>Open Project</button>
                    </li>
                    <li></li>
                    <li>
                        <button onclick={handleCloseProject}>Close Project</button>
                    </li>
                </ul>
            </div>
        </div>
    {:else}
        <div class="flex items-center h-full ms-1">
            <button
                class="btn btn-ghost btn-xs btn-square"
                onclick={toggleMenu}
                aria-label="Menu"
            >
                <MenuIcon class="h-4 w-4" />
            </button>
            {#if projectName}
                <span class="text-xs opacity-50 ms-1">{projectName}</span>
            {/if}
        </div>
    {/if}

    <!-- draggable title area -->
    <div
        class="flex-1 h-full"
        data-tauri-drag-region
        role="button"
        tabindex="-1"
    ></div>

    <!-- right: window controls -->
    <div class="flex items-center h-full">
        <button
            class="btn btn-ghost btn-xs btn-square h-full rounded-none w-8"
            onclick={minimize}
            aria-label="Minimize"
        >
            <Minus class="h-4 w-4" />
        </button>
        <button
            class="btn btn-ghost btn-xs btn-square h-full rounded-none"
            onclick={toggleMaximize}
            aria-label="Maximize"
        >
            {#if maximized}
                <Unmaximize class="h-3.5 w-3.5" />
            {:else}
                <Square class="h-3 w-3" />
            {/if}
        </button>
        <button
            class="btn btn-ghost btn-xs btn-square h-full rounded-none w-8 hover:bg-error hover:text-error-content"
            onclick={closeWindow}
            aria-label="Close"
        >
            <X class="h-4 w-4" />
        </button>
    </div>
</div>

<!-- about modal -->
<AboutDialog getDialog={(d) => (aboutDialog = d)} />
