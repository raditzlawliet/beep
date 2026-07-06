<script lang="ts">
    import { app } from "$lib/app-state.svelte";
    import { PlusIcon, FolderOpenIcon } from "@lucide/svelte";

    interface Props {
        onNewRequest: () => void;
        onOpenProject: () => void;
    }

    let { onNewRequest, onOpenProject }: Props = $props();
</script>

<div class="flex-1 flex items-center justify-center">
    <div class="flex flex-col gap-6 w-80 select-none">
        <!-- Header: icon + title -->
        <div class="flex items-center justify-center gap-4">
            <img src="/icon.png" alt="Beep" class="w-10 h-10 opacity-80 shrink-0" />
            <div>
                <div class="text-lg">Welcome back to Beep</div>
                <div class="text-sm italic font-thin text-neutral-content">The lightweight intuitive API client</div>
            </div>
        </div>

        <!-- Get Started -->
        <div>
            <div class="divider divider-start text-xs text-neutral-content uppercase m-0 my-2">Get Started</div>
            <div class="flex flex-col">
                <button class="btn btn-xs btn-ghost justify-start gap-2 w-full h-8" onclick={onNewRequest}>
                    <PlusIcon class="w-3.5 h-3.5 text-neutral-content" />
                    <span class="flex-1 text-start font-normal">New Request</span>
                    <span class="text-neutral-content font-normal text-xs opacity-50 ">{app.modKey}+N</span>
                </button>
                <button class="btn btn-xs btn-ghost justify-start gap-2 w-full h-8" onclick={onOpenProject}>
                    <FolderOpenIcon class="w-3.5 h-3.5 text-neutral-content" />
                    <span class="flex-1 text-start font-normal">Open Project</span>
                    <span class="text-neutral-content font-normal text-xs opacity-50">{app.modKey}+O</span>
                </button>
            </div>
        </div>
    </div>
</div>
