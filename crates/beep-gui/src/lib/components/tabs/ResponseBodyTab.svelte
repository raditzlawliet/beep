<script lang="ts">
    import type { HttpResponse } from "$lib/types";
    import CodeEditor from "$lib/components/CodeEditor.svelte";
    import { ChevronDownIcon, CopyIcon, TextWrapIcon } from "@lucide/svelte";

    interface Props {
        response: HttpResponse;
    }

    let { response }: Props = $props();

    type BodyDisplay = "html" | "json" | "raw";
    type CodeLanguage = "html" | "json" | "text";
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
        return "raw";
    }

    $effect(() => {
        bodyDisplay = detectDisplayFromContentType(
            response.headers["content-type"] || "",
        );
    });

    function formatBody(body: string): string {
        try {
            return JSON.stringify(JSON.parse(body), null, 2);
        } catch {
            return body;
        }
    }

    let displayBody = $derived(
        bodyDisplay === "json" ? formatBody(response.body) : response.body,
    );

    let bodyDisplayLabel = $derived(
        bodyDisplay === "html"
            ? "HTML"
            : bodyDisplay === "json"
              ? "JSON"
              : "Raw",
    );

    let codeLanguage = $derived<CodeLanguage>(
        bodyDisplay === "html"
            ? "html"
            : bodyDisplay === "json"
              ? "json"
              : "text",
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
                    {bodyDisplayLabel}
                    {#if activeTab === "body"}
                        <ChevronDownIcon class="w-3 h-3" />
                    {/if}
                </button>
                {#if bodyDropdownOpen}
                    <ul
                        class="dropdown-content menu menu-sm bg-base-200 rounded-box z-50 shadow-sm border border-base-content/10 w-16 p-1"
                    >
                        <li>
                            <button onclick={() => selectBodyDisplay("html")}
                                >HTML</button
                            >
                        </li>
                        <li>
                            <button onclick={() => selectBodyDisplay("json")}
                                >JSON</button
                            >
                        </li>
                        <li class="m-0 p-0 my-1"></li>
                        <li>
                            <button onclick={() => selectBodyDisplay("raw")}
                                >Raw</button
                            >
                        </li>
                    </ul>
                {/if}
            </div>
            <button
                role="tab"
                class="tab"
                class:tab-active={activeTab === "preview"}
                onclick={selectPreview}
            >
                Preview
            </button>
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
            <iframe
                class="w-full h-full border-0"
                srcdoc={displayBody}
                sandbox="allow-scripts"
                title="Response preview"
            ></iframe>
        {/if}
    </div>
</div>
