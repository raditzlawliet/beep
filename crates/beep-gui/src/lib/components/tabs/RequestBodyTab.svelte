<script lang="ts">
    import type { FormField } from "$lib/types";
    import CodeEditor from "$lib/components/CodeEditor.svelte";
    import RequestUrlEncodedTab from "./RequestUrlEncodedTab.svelte";
    import RequestMultipartTab from "./RequestMultipartTab.svelte";
    import type { BodyMode } from "$lib/components/RequestForm.svelte";
    import { BracesIcon, ChevronDownIcon, PaperclipIcon, LinkIcon, FileTextIcon, FileCodeIcon, XIcon, CheckIcon, CodeXmlIcon } from "@lucide/svelte";

    interface Props {
        bodyMode: BodyMode;
        rawBodyContent: string;
        formUrlEncoded: FormField[];
        formMultipart: FormField[];
        onBodyModeChange: (mode: BodyMode) => void;
        onRawBodyChange: (value: string) => void;
        onBeautify: () => Promise<string>;
        onFormUrlEncodedChange: (fields: FormField[]) => void;
        onFormMultipartChange: (fields: FormField[]) => void;
    }

    let {
        bodyMode,
        rawBodyContent,
        formUrlEncoded,
        formMultipart,
        onBodyModeChange,
        onRawBodyChange,
        onBeautify,
        onFormUrlEncodedChange,
        onFormMultipartChange,
    }: Props = $props();

    // Derived: body type extracted from combined bodyMode ("raw/json" → "json").
    const bodyType = $derived(
        bodyMode.startsWith("raw/") ? bodyMode.slice(4) : "text",
    );

    const selectedLabel = $derived(
        bodyMode === "form-multipart" ? "Multipart Form"
        : bodyMode === "form-urlencoded" ? "Form URL Encoded"
        : bodyMode === "raw/json" ? "JSON"
        : bodyMode === "raw/xml" ? "XML"
        : bodyMode === "raw/html" ? "HTML"
        : bodyMode === "raw/text" ? "Text"
        : "No Body"
    );

    const selectedValue = $derived(bodyMode);

    function select(val: string) {
        onBodyModeChange(val as BodyMode);
        (document.activeElement as HTMLElement)?.blur();
    }
</script>

<div class="flex flex-col flex-1">
    <div class="flex items-center p-0 px-2 min-h-4">
        <div class="dropdown">
            <button class="btn btn-ghost btn-xs font-normal gap-1 align-baseline" role="menu" tabindex="0">
                {#if bodyMode === "form-multipart"}
                    <PaperclipIcon class="w-3 h-3" />
                {:else if bodyMode === "form-urlencoded"}
                    <LinkIcon class="w-3 h-3" />
                {:else if bodyMode.startsWith("raw/")}
                    {#if bodyType === "json"}
                        <BracesIcon class="w-3 h-3" />
                    {:else if bodyType === "xml"}
                        <FileCodeIcon class="w-3 h-3" />
                    {:else if bodyType === "html"}
                        <CodeXmlIcon class="w-3 h-3" />
                    {:else}
                        <FileTextIcon class="w-3 h-3" />
                    {/if}
                {:else}
                    <XIcon class="w-3 h-3" />
                {/if}
                {selectedLabel}
                <ChevronDownIcon class="w-3 h-3" />
            </button>
            <ul class="dropdown-content menu menu-sm bg-base-200 rounded-box z-2 shadow-sm border border-base-content/10 w-52 p-1 gap-0.5 fixed" tabindex="-1">
                {#snippet item(val: string)}
                    <CheckIcon class="w-3 h-3 ml-auto {selectedValue === val ? '' : 'invisible'}" />
                {/snippet}
                <li class="menu-title p-0 px-2 text-xs">Form</li>
                <li>
                    <button onclick={() => select("form-multipart")}>
                        <PaperclipIcon class="w-3.5 h-3.5" /> Multipart Form
                        {@render item("form-multipart")}
                    </button>
                </li>
                <li>
                    <button onclick={() => select("form-urlencoded")}>
                        <LinkIcon class="w-3.5 h-3.5" /> Form URL Encoded
                        {@render item("form-urlencoded")}
                    </button>
                </li>
                <li class="menu-title p-0 px-2 text-xs">Raw</li>
                <li>
                    <button onclick={() => select("raw/json")}>
                        <BracesIcon class="w-3.5 h-3.5" /> JSON
                        {@render item("raw/json")}
                    </button>
                </li>
                <li>
                    <button onclick={() => select("raw/xml")}>
                        <FileCodeIcon class="w-3.5 h-3.5" /> XML
                        {@render item("raw/xml")}
                    </button>
                </li>
                <li>
                    <button onclick={() => select("raw/html")}>
                        <CodeXmlIcon class="w-3.5 h-3.5" /> HTML
                        {@render item("raw/html")}
                    </button>
                </li>
                <li>
                    <button onclick={() => select("raw/text")}>
                        <FileTextIcon class="w-3.5 h-3.5" /> Text
                        {@render item("raw/text")}
                    </button>
                </li>
                <li class="menu-title p-0 px-2 text-xs">Other</li>
                <li>
                    <button onclick={() => select("none")}>
                        <XIcon class="w-3.5 h-3.5" /> No Body
                        {@render item("none")}
                    </button>
                </li>
            </ul>
        </div>

        <div class="flex-1"></div>
        {#if bodyMode.startsWith("raw/") && (bodyType === "json" || bodyType === "html" || bodyType === "xml")}
            <button class="btn btn-xs btn-ghost text-accent"
                onclick={async () => { const beautified = await onBeautify(); onRawBodyChange(beautified); }}
                title={bodyType === "json" ? "Beautify JSON" : bodyType === "html" ? "Beautify HTML" : "Beautify XML"}>
                <BracesIcon class="h-3.5 w-3.5" />Beautify
            </button>
        {/if}
    </div>

    {#if bodyMode.startsWith("raw/")}
        <CodeEditor
            value={rawBodyContent}
            language={bodyType === "json" ? "json" : bodyType === "html" ? "html" : bodyType === "xml" ? "xml" : "text"}
            onchange={(v: string) => onRawBodyChange(v)}
            class="border border-base-300 rounded-md overflow-hidden flex-1 min-h-0"
        />
    {/if}

    {#if bodyMode === "form-urlencoded"}
        <div class="flex-1 min-h-0 overflow-y-auto">
            <RequestUrlEncodedTab initialValue={formUrlEncoded} onchange={onFormUrlEncodedChange} />
        </div>
    {/if}

    {#if bodyMode === "form-multipart"}
        <div class="flex-1 min-h-0 overflow-y-auto">
            <RequestMultipartTab initialValue={formMultipart} onchange={onFormMultipartChange} />
        </div>
    {/if}
</div>
