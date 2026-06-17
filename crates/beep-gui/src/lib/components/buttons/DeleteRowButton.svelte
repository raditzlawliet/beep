<script lang="ts">
    import { XIcon } from "@lucide/svelte";

    interface Props {
        onclick: () => void;
    }

    let { onclick }: Props = $props();
</script>

<button
    class="btn btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100"
    {onclick}
>
    <XIcon class="w-3 h-3" />
</button>
