<script lang="ts">
    import { PlusIcon } from "@lucide/svelte";

    interface Props {
        onclick: () => void;
        text: string;
    }

    let { onclick, text }: Props = $props();
</script>

<button
    class="btn btn-ghost btn-xs gap-1 text-xs font-normal opacity-60 hover:opacity-100"
    {onclick}
>
    <PlusIcon class="w-3 h-3" />
    {text}
</button>
