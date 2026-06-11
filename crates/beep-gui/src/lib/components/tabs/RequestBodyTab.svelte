<script lang="ts">
    import CodeEditor from "$lib/components/CodeEditor.svelte";
    import type {
        BodyMode,
        BodyType,
    } from "$lib/components/RequestForm.svelte";
    import { BracesIcon, ChevronDownIcon } from "@lucide/svelte";

    interface Props {
        bodyMode: BodyMode;
        bodyType: BodyType;
        rawBodyContent: string;
        onBodyModeChange: (mode: BodyMode) => void;
        onBodyTypeChange: (type: BodyType) => void;
        onRawBodyChange: (value: string) => void;
        onBeautify: () => string;
    }

    let {
        bodyMode,
        bodyType,
        rawBodyContent,
        onBodyModeChange,
        onBodyTypeChange,
        onRawBodyChange,
        onBeautify,
    }: Props = $props();
</script>

<div class="flex flex-col flex-1">
    <!-- radio buttons for body mode -->
    <div class="flex items-center gap-4 px-2 min-h-6 mb-2">
        <label class="flex items-center gap-1.5 cursor-pointer">
            <input
                type="radio"
                name="bodyMode"
                class="radio radio-xs"
                value="none"
                checked={bodyMode === "none"}
                onchange={() => onBodyModeChange("none")}
            />
            <span class="text-xs">None</span>
        </label>
        <label class="flex items-center gap-1.5 cursor-pointer">
            <input
                type="radio"
                name="bodyMode"
                class="radio radio-xs"
                value="raw"
                checked={bodyMode === "raw"}
                onchange={() => onBodyModeChange("raw")}
            />
            <span class="text-xs">Raw</span>
        </label>

        {#if bodyMode === "raw"}
            <div class="dropdown">
                <button
                    class="btn btn-ghost btn-xs font-normal"
                    role="menu"
                    tabindex="0"
                >
                    {bodyType}
                    <ChevronDownIcon class="w-3 h-3" />
                </button>
                <ul
                    class="dropdown-content menu menu-sm bg-base-200 rounded-box z-1 shadow-sm border border-base-content/10 w-16 p-1 gap-1"
                    tabindex="-1"
                >
                    <li>
                        <button
                            onclick={() => {
                                onBodyTypeChange("text");
                                (document.activeElement as HTMLElement)?.blur();
                            }}>Text</button
                        >
                    </li>
                    <li>
                        <button
                            onclick={() => {
                                onBodyTypeChange("json");
                                (document.activeElement as HTMLElement)?.blur();
                            }}>JSON</button
                        >
                    </li>
                </ul>
            </div>
            <div class="flex-1"></div>
            {#if bodyType === "json"}
                <button
                    class="btn btn-xs btn-ghost text-accent"
                    onclick={() => {
                        const beautified = onBeautify();
                        onRawBodyChange(beautified);
                    }}
                    title="Beautify JSON"
                >
                    <BracesIcon class="h-3.5 w-3.5" />Beautify
                </button>
            {/if}
        {/if}
    </div>

    <!-- raw editor -->
    {#if bodyMode === "raw"}
        <CodeEditor
            value={rawBodyContent}
            language={bodyType === "json" ? "json" : "text"}
            onchange={(v: string) => onRawBodyChange(v)}
            class="border border-base-300 rounded-md overflow-hidden flex-1 min-h-0"
        />
    {/if}
</div>
