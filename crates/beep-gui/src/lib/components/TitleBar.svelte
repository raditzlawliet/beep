<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { X, Minus, Square, MenuIcon } from "@lucide/svelte";
  import Unmaximize from "$lib/components/icons/Unmaximize.svelte";
  import AboutDialog from "$lib/components/modals/AboutDialog.svelte";
  import NewRequestPopup from "$lib/components/NewRequestPopup.svelte";
  import { setNewPopupHandler } from "$lib/hotkeys.svelte";
  import { app } from "$lib/app-state.svelte";

  interface Props {
    activeFileName: string | null;
    projectName: string | null;
    hasActiveHttpTab: boolean;
    hasActiveFileTab: boolean;
    hasUnsavedTabs: boolean;
    //
    onNewUntitled: () => void;
    onNewInFile: () => void;
    onNewRequest: () => void;
    onOpenProject: () => void;
    onCloseProject: () => void;
    onCloseTab: () => void;
    onSave: () => void;
    onSaveAll: () => void;
  }

  let {
    activeFileName,
    projectName,
    hasActiveHttpTab,
    hasActiveFileTab,
    hasUnsavedTabs,
    //
    onNewUntitled,
    onNewInFile,
    onNewRequest,
    onOpenProject,
    onCloseProject,
    onCloseTab,
    onSave,
    onSaveAll,
  }: Props = $props();

  let aboutDialog = $state<HTMLDialogElement | null>(null);
  let menuOpen = $state(false);
  let maximized = $state(false);
  let newPopupOpen = $state(false);

  function toggleMenu(e: MouseEvent) {
    menuOpen = !menuOpen;
    e.stopPropagation();
  }

  function closeAll() {
    menuOpen = false;
    newPopupOpen = false;
  }

  function openNewRequestPopup(e: Event) {
    closeAll();
    e.stopPropagation();
    newPopupOpen = true;
  }

  function handleNewUntitled() {
    onNewUntitled();
    closeAll();
  }

  function handleNewInFile() {
    onNewInFile();
    closeAll();
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

  function handleCloseTab() {
    onCloseTab();
    closeAll();
  }

  function handleSave() {
    onSave();
    closeAll();
  }

  function handleSaveAll() {
    onSaveAll();
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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      closeAll();
    }
  }

  function handleWindowClick() {
    if (menuOpen || newPopupOpen) closeAll();
  }

  const hasActiveTab = $derived(hasActiveHttpTab || hasActiveFileTab);

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

  // Register Ctrl+N open NewRequestPopup
  $effect(() => {
    setNewPopupHandler(() => {
      closeAll();
      newPopupOpen = true;
    });
    return () => setNewPopupHandler(null);
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
          <li><button onclick={quit}>
            <span>Quit</span>
            <span class="text-xs opacity-50 ml-auto">{app.modKey}+Q</span>
          </button></li>
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
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <button onclick={openNewRequestPopup}>
              <span>New</span>
              <span class="text-xs opacity-50 ml-auto">{app.modKey}+N</span>
            </button>
          </li>
          <li>
            <button onclick={handleNewRequest}>
              <span>New Request</span>
              <span class="text-xs opacity-50 ml-auto">{app.modKey}+Shift+N</span>
            </button>
          </li>
          <li></li>
          <li>
            <button onclick={handleOpenProject}>
                <span>Open Project...</span>
                <span class="text-xs opacity-50 ml-auto">{app.modKey}+O</span>
            </button>
          </li>
          <li></li>
          <li>
            <button onclick={handleSave} disabled={!hasActiveTab}>
              <span>Save</span>
              <span class="text-xs opacity-50 ml-auto">{app.modKey}+S</span>
            </button>
          </li>
          <li>
            <button onclick={handleSaveAll} disabled={!hasUnsavedTabs}>
                <span>Save All</span>
                <span class="text-xs opacity-50 ml-auto flex gap-2">
                    <span>{app.modKey}+K</span>
                    <span>S</span>
                </span>
            </button>
          </li>
          <li></li>
          <li>
            <button onclick={handleCloseTab} disabled={!hasActiveTab}>
              <span>Close Tab</span>
              <span class="text-xs opacity-50 ml-auto">{app.modKey}+W</span>
            </button>
          </li>
          <li>
            <button onclick={handleCloseProject}>
              <span>Close Project</span>
            </button>
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
        <MenuIcon class="h-3.5 w-3.5" />
      </button>
      {#if projectName}
        <span class="text-xs ms-1">{projectName}</span>
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

<NewRequestPopup
    open={newPopupOpen}
    {activeFileName}
    onNewUntitled={handleNewUntitled}
    onNewInFile={handleNewInFile}
    onClose={() => (newPopupOpen = false)}
/>

<!-- about modal -->
<AboutDialog getDialog={(d) => (aboutDialog = d)} />
