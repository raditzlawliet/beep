<script lang="ts">
    import "./layout.css";
    import "./style.css";

    import Toast from "$lib/components/Toast.svelte";
    import { getToastState, closeToast } from "$lib/toast.svelte";
    import { onHistoryChanged, onStatusChanged } from "$lib/events.svelte";

    const { children } = $props();
    const toast = getToastState();

    // register Tauri event listeners (placeholders — backend may not emit yet)
    $effect(() => {
        onHistoryChanged(() => {
          // TODO
        });

        onStatusChanged((payload) => {
          // TODO
        });
    });
</script>

{@render children()}

<Toast title={toast.title} description={toast.description} id={toast.id} onClose={closeToast} />
