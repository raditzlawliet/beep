<script lang="ts">
    import { fly } from "svelte/transition";
    import { CheckIcon, CircleXIcon, CopyIcon, XIcon } from "@lucide/svelte";

    interface Props {
        title: string;
        description: string;
        id: number;
        onClose: () => void;
    }

    let { title, description, id, onClose }: Props = $props();
    let copied = $state(false);
    let expanded = $state(false);

    function handleExpand() {
        expanded = true;
    }

    async function handleCopy() {
        try {
            await navigator.clipboard.writeText(description);
            copied = true;
            setTimeout(() => (copied = false), 2000);
        } catch {
            // clipboard unavailable
        }
    }

    $effect(() => {
        id;
        expanded = false;
        copied = false;
    });
</script>

{#if title}
    <div
        class="toast toast-end toast-bottom z-50 pointer-events-none"
        transition:fly={{ y: 20, duration: 200 }}
    >
        <div
            role="alert"
            class="alert alert-error alert-soft shadow-lg pointer-events-auto max-w-3xl"
        >
            <CircleXIcon class="h-4 w-4" />
            <div class="flex-1 min-w-0">
                <div class="font-medium text-sm">{title}</div>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                    class="text-xs opacity-80"
                    class:cursor-pointer={!expanded}
                    class:select-none={!expanded}
                    class:line-clamp-1={!expanded}
                    class:whitespace-pre-wrap={expanded}
                    class:break-all={expanded}
                    onclick={handleExpand}
                    title={expanded ? "" : "Click to expand"}
                >
                    {description}
                </div>
            </div>
            <div class="flex gap-0.5">
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    onclick={handleCopy}
                    aria-label="Copy message"
                    title="Copy message"
                >
                    {#if copied}
                        <CheckIcon class="h-3.5 w-3.5 text-success" />
                    {:else}
                        <CopyIcon class="h-3.5 w-3.5" />
                    {/if}
                </button>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    onclick={onClose}
                    aria-label="Close"
                >
                    <XIcon class="h-3.5 w-3.5" />
                </button>
            </div>
        </div>
    </div>
{/if}
