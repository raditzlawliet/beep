<script lang="ts">
    import { PlayIcon } from "@lucide/svelte";

    interface Props {
        pos: number;
        onClick: (pos: number, e: MouseEvent) => void;
    }

    let { pos, onClick }: Props = $props();

    function handleClick(e: MouseEvent) {
        e.stopPropagation();
        e.preventDefault();
        onClick(pos, e);
    }
</script>

<button
    class="cm-run-marker cursor-pointer border-none text-base-content"
    title="Send request"
    onclick={handleClick}
>
    <PlayIcon class="w-2 h-2"></PlayIcon>
</button>
