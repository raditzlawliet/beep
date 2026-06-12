<script lang="ts">
    import "./layout.css";
    import "./style.css";
    import Toast from "$lib/components/Toast.svelte";
    import { getToastState, closeToast } from "$lib/toast.svelte";

    const { children } = $props();
    const toast = getToastState();
</script>

{@render children()}

<Toast title={toast.title} description={toast.description} id={toast.id} onClose={closeToast} />
