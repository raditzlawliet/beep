<script lang="ts">
    import { TrashIcon } from "@lucide/svelte";

    interface Props {
        onclick: () => void;
        /** Tooltip + aria label. */
        label?: string;
    }

    let { onclick, label = "Delete row" }: Props = $props();
</script>

<button
    class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 tooltip tooltip-left"
    {onclick}
    aria-label={label}>
    <TrashIcon class="w-3 h-3" />
    <div class="tooltip-content text-xs">{label}</div>
</button>
