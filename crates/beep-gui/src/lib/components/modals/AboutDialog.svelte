<script lang="ts">
    let { getDialog }: { getDialog: (d: HTMLDialogElement) => void } = $props();
    let dialog = $state<HTMLDialogElement | null>(null);

    const version = import.meta.env.VITE_BEEP_VERSION;
    const gitSha = import.meta.env.VITE_GIT_SHA;

    $effect(() => {
        if (dialog) getDialog(dialog);
    });
</script>

<dialog class="modal" bind:this={dialog}>
    <div class="modal-box border border-base-content/10 text-center">
        <div>
            Beep
            <span class="text-sm">{version}</span>
        </div>
        <div class="text-xs opacity-50 mt-1">Intuitive API Client</div>
        <div class="text-[10px] opacity-50 mt-5 font-mono">sha</div>
        <div class="text-xs mt-1 font-mono">{gitSha}</div>
    </div>
    <form method="dialog" class="modal-backdrop">
        <button>Ok</button>
    </form>
</dialog>
