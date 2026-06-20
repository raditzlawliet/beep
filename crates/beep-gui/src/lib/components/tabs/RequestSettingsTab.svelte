<script lang="ts">
    import type { HttpVersion } from "$lib/types";
    import HistorySidebar from "../HistorySidebar.svelte";
    import HorizontalDivider from "../uis/HorizontalDivider.svelte";

    interface Props {
        httpVersion: HttpVersion;
        onUpdate: (v: HttpVersion) => void;
    }

    let { httpVersion, onUpdate }: Props = $props();

    const HTTP_VERSIONS: HttpVersion[] = ["Auto", "Http1", "Http2"];
    const VERSION_LABELS: Record<HttpVersion, string> = {
        Auto: "Auto",
        Http1: "HTTP/1.x",
        Http2: "HTTP/2",
    };
</script>

<div class="px-2.5 space-y-2">
    <!-- HTTP Version -->
    <div class="flex items-center gap-4">
        <div class="flex-1">
            <label class="label px-0" for="http-version-select">
                <span class="label-text text-xs font-medium">HTTP version</span>
            </label>
            <p class="text-xs text-base-content/50">
                Select the HTTP version to use for sending the request.
            </p>
        </div>
        <select
            id="http-version-select"
            class="select select-bordered select-sm w-32"
            value={httpVersion}
            onchange={(e) =>
                onUpdate(
                    (e.target as HTMLSelectElement).value as HttpVersion,
                )}
        >
            {#each HTTP_VERSIONS as v}
                <option value={v}>{VERSION_LABELS[v]}</option>
            {/each}
        </select>
    </div>
</div>
