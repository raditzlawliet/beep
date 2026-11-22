<script lang="ts">
    import { CheckIcon } from "@lucide/svelte";
    // Primary dot for the selected env; grows into an "active" badge while a `group/active` parent is hovered.
    let { class: cls = "" }: { class?: string } = $props();
</script>

<!-- fixed h-4 box: the pill grows inside it, so the row height never changes -->
<span class="inline-flex h-4 shrink-0 items-center {cls}" aria-label="Active environment">
    <span
        class="inline-flex items-center overflow-hidden whitespace-nowrap rounded-full bg-primary text-primary-content text-[10px] leading-none font-medium
            h-1.5 max-w-1.5 min-w-1.5 px-0 transition-all duration-200
            group-hover/active:h-4 group-hover/active:max-w-16 group-hover/active:px-1.5"
    >
        <span class="inline-flex items-center gap-0.5 opacity-0 transition-opacity duration-200 group-hover/active:opacity-100"><CheckIcon class="h-2.5 w-2.5" />active</span>
    </span>
</span>
