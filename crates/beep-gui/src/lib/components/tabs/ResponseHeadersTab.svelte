<script lang="ts">
    import type { HttpResponse } from "$lib/types";

    interface Props {
        response: HttpResponse;
    }

    let { response }: Props = $props();

    function headerRows(resp: HttpResponse): { key: string; value: string }[] {
        return Object.entries(resp.headers).map(([key, value]) => ({
            key,
            value,
        }));
    }

    let expandedRow = $state<number | null>(null);

    function expandRow(idx: number) {
        expandedRow = idx;
    }

    function collapseRow() {
        expandedRow = null;
    }
</script>

<table class="table table-xs table-pin-rows table-pin-cols w-full table-fixed">
    <thead>
        <tr>
            <th class="text-xs w-72">Key</th>
            <th class="text-xs">Value</th>
        </tr>
    </thead>
    <tbody>
        {#each headerRows(response) as row, i (i)}
            {@const isExpanded = expandedRow === i}
            <tr class="hover:bg-base-300 divide-x divide-base-content/10">
                <td
                    class="cursor-pointer"
                    class:align-top={isExpanded}
                    onclick={() => expandRow(i)}
                    onfocusout={() => collapseRow()}
                    title={row.key}
                >
                    <span
                        class="text-xs font-mono font-semibold block"
                        class:truncate={!isExpanded}
                        class:break-all={isExpanded}
                    >
                        {row.key}
                    </span>
                </td>
                <td
                    class="cursor-pointer"
                    class:align-top={isExpanded}
                    onclick={() => expandRow(i)}
                    onfocusout={() => collapseRow()}
                    title={row.value}
                >
                    <span
                        class="text-xs font-mono block"
                        class:truncate={!isExpanded}
                        class:break-all={isExpanded}
                    >
                        {row.value}
                    </span>
                </td>
            </tr>
        {/each}
    </tbody>
</table>
