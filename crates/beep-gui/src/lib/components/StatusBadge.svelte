<script lang="ts">
    import { statusTextColor, statusText } from "$lib/types";

    interface Props {
        status: number;
        showLabel?: boolean;
    }

    let { status, showLabel = true }: Props = $props();
</script>

<span class="font-mono font-bold {statusTextColor(status)}">
    {status}
    {#if showLabel && statusText(status)}
        <span class="opacity-70 font-normal">{statusText(status)}</span>
    {/if}
</span>
