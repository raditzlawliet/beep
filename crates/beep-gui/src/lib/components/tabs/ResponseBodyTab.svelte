<script lang="ts">
    import type { HttpResponse } from "$lib/types";
    import CodeEditor from "$lib/components/CodeEditor.svelte";
    import XmlTreeView from "$lib/components/XmlTreeView.svelte";
    import { BracesIcon, CheckIcon, ChevronDownIcon, CodeXmlIcon, CopyIcon, FileCodeIcon, FileTextIcon, TextWrapIcon } from "@lucide/svelte";

    interface Props {
        response: HttpResponse;
    }

    let { response }: Props = $props();

    type BodyDisplay = "html" | "json" | "xml" | "raw" | "base64";
    type CodeLanguage = "html" | "json" | "xml" | "text";
    type ActiveTab = "body" | "preview";

    let bodyDisplay = $state<BodyDisplay>("raw");
    let activeTab = $state<ActiveTab>("body");
    let bodyDropdownOpen = $state(false);
    let wrapLines = $state(true);

    function detectDisplayFromContentType(ct: string): BodyDisplay {
        const t = ct.toLowerCase();
        if (t.includes("text/html")) return "html";
        if (t.includes("application/json") || t.includes("+json"))
            return "json";
        if (
            t.includes("application/xml") ||
            t.includes("text/xml") ||
            t.includes("+xml")
        )
            return "xml";
        return "raw";
    }

    $effect(() => {
        bodyDisplay = detectedDisplay;
    });

    function formatBody(body: string): string {
        try {
            return JSON.stringify(JSON.parse(body), null, 2);
        } catch {
            return body;
        }
    }

    let bodyDisplayLabel = $derived(
        bodyDisplay === "html"
            ? "HTML"
            : bodyDisplay === "json"
              ? "JSON"
              : bodyDisplay === "xml"
                ? "XML"
                : bodyDisplay === "base64"
                  ? "Base64"
                  : "Raw",
    );

    let codeLanguage = $derived<CodeLanguage>(
        bodyDisplay === "html"
            ? "html"
            : bodyDisplay === "json"
              ? "json"
              : bodyDisplay === "xml"
                ? "xml"
                : "text",
    );

    let detectedDisplay = $derived(
        detectDisplayFromContentType(response.headers["content-type"] || ""),
    );

    let isImage = $derived(
        (response.headers["content-type"] || "").startsWith("image/")
        && response.body_encoding === "base64",
    );

    let isBinary = $derived(response.body_encoding === "base64");

    let hidePreview = $derived(isBinary && !isImage);

    $effect(() => {
        if (hidePreview && activeTab === "preview") {
            activeTab = "body";
        }
    });

    let binaryPreview = $derived.by(() => {
        if (!isBinary) return "";
        try {
            return atob(response.body);
        } catch {
            return response.body;
        }
    });

    let displayBody = $derived(
        bodyDisplay === "json" ? formatBody(response.body)
        : bodyDisplay === "base64" ? response.body
        : isBinary ? binaryPreview
        : response.body,
    );

    function selectBodyDisplay(mode: BodyDisplay) {
        bodyDisplay = mode;
        bodyDropdownOpen = false;
    }

    function handleBodyTabClick() {
        if (activeTab === "body") {
            bodyDropdownOpen = !bodyDropdownOpen;
        } else {
            activeTab = "body";
        }
    }

    function selectPreview() {
        activeTab = "preview";
    }

    async function copyBody() {
        try {
            await navigator.clipboard.writeText(displayBody);
        } catch {
            // Fallback ignored
        }
    }
</script>

<div class="flex flex-col min-h-0 h-full">
    <!-- Toolbar -->
    <div
        class="flex items-center px-2 py-1 border-b border-base-300 bg-base-200/50"
    >
        <!-- Tabs -->
        <div role="tablist" class="tabs tabs-bordered tabs-xs">
            <div class="dropdown">
                <button
                    role="tab"
                    class="tab gap-1 text-xs"
                    class:tab-active={activeTab === "body"}
                    onclick={handleBodyTabClick}
                >
                    {#if bodyDisplay === "json"}
                        <BracesIcon class="w-3 h-3" />
                    {:else if bodyDisplay === "xml"}
                        <FileCodeIcon class="w-3 h-3" />
                    {:else if bodyDisplay === "html"}
                        <CodeXmlIcon class="w-3 h-3" />
                    {:else}
                        <FileTextIcon class="w-3 h-3" />
                    {/if}
                    {bodyDisplayLabel}
                    {#if activeTab === "body"}
                        <ChevronDownIcon class="w-3 h-3" />
                    {/if}
                </button>
                {#if bodyDropdownOpen}
                    {#snippet check(mode: BodyDisplay)}
                        <CheckIcon class="w-3 h-3 ml-auto {bodyDisplay === mode ? '' : 'invisible'}" />
                    {/snippet}
                    <ul
                        class="dropdown-content menu menu-sm bg-base-200 rounded-box z-50 shadow-sm border border-base-content/10 w-28 p-1"
                    >
                        {#if !isBinary}
                        <li>
                            <button onclick={() => selectBodyDisplay("json")}>
                                <BracesIcon class="w-3.5 h-3.5" /> JSON
                                {@render check("json")}
                            </button>
                        </li>
                        <li>
                            <button onclick={() => selectBodyDisplay("xml")}>
                                <FileCodeIcon class="w-3.5 h-3.5" /> XML
                                {@render check("xml")}
                            </button>
                        </li>
                        <li>
                            <button onclick={() => selectBodyDisplay("html")}>
                                <CodeXmlIcon class="w-3.5 h-3.5" /> HTML
                                {@render check("html")}
                            </button>
                        </li>
                        <li class="m-0 p-0 my-1"></li>
                        {/if}
                        <li>
                            <button onclick={() => selectBodyDisplay("raw")}>
                                <FileTextIcon class="w-3.5 h-3.5" /> Raw
                                {@render check("raw")}
                            </button>
                        </li>
                        <li>
                            <button onclick={() => selectBodyDisplay("base64")}>
                                <FileTextIcon class="w-3.5 h-3.5" /> Base64
                                {@render check("base64")}
                            </button>
                        </li>
                    </ul>
                {/if}
            </div>
            {#if !hidePreview}
            <button
                role="tab"
                class="tab"
                class:tab-active={activeTab === "preview"}
                onclick={selectPreview}
            >
                Preview
            </button>
            {/if}
        </div>

        <!-- Spacer -->
        <div class="flex-1"></div>

        <!-- Right: action buttons -->
        {#if activeTab === "body"}
            <div class="flex items-center gap-0.5">
                <button
                    class="btn btn-xs btn-soft btn-square"
                    class:btn-active={!wrapLines}
                    onclick={() => (wrapLines = !wrapLines)}
                    title="Toggle line wrap"
                >
                    <TextWrapIcon class="w-3 h-3"></TextWrapIcon>
                </button>
                <button
                    class="btn btn-xs btn-ghost btn-square"
                    onclick={copyBody}
                    title="Copy body"
                >
                    <CopyIcon class="w-3 h-3"></CopyIcon>
                </button>
            </div>
        {/if}
    </div>

    <!-- Content area -->
    <div class="flex-1 min-h-0 overflow-auto">
        {#if activeTab === "body"}
            <CodeEditor
                value={displayBody}
                language={codeLanguage}
                readonly={true}
                {wrapLines}
                class="border-0 rounded-none h-full"
            />
        {:else}
            <!-- viewer -->
            {#if bodyDisplay === "xml"}
                <XmlTreeView xml={response.body} />
            {:else if isImage}
                <div class="flex items-center justify-center h-full p-4 bg-[repeating-conic-gradient(#80808010_0%_25%,transparent_0%_50%)_50%/20px_20px]">
                    <img
                        src="data:{response.headers['content-type']};base64,{response.body}"
                        alt=""
                        class="max-w-full max-h-full object-contain"
                    />
                </div>
            {:else}
            <iframe
                class="w-full h-full border-0"
                srcdoc={displayBody}
                sandbox="allow-scripts"
                title="Response preview"
            ></iframe>
            {/if}
        {/if}
    </div>
</div>
