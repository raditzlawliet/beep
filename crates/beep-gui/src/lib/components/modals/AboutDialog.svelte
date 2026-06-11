<script lang="ts">
    let { getDialog }: { getDialog: (d: HTMLDialogElement) => void } = $props();
    let dialog = $state<HTMLDialogElement | null>(null);

    $effect(() => {
        if (dialog) getDialog(dialog);
    });
</script>

<dialog class="modal" bind:this={dialog}>
    <div class="modal-box border border-base-content/10 text-center">
        <div>
            Beep
            <span class="text-sm">0.1.0</span>
        </div>
        <div class="text-xs opacity-50">Intuitive API Client</div>
    </div>
    <form method="dialog" class="modal-backdrop">
        <button>Ok</button>
    </form>
</dialog>
