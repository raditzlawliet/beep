<script lang="ts">
    import { TrashIcon } from "@lucide/svelte";

    interface Props {
        onclick: () => void;
    }

    let { onclick }: Props = $props();
</script>

<button
    class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100"
    {onclick}
    aria-label="Delete row">
    <TrashIcon class="w-3 h-3" />
</button>
