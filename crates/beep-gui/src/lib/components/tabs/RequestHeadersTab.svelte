<script lang="ts">
    import { EyeIcon, EyeOffIcon } from "@lucide/svelte";
    import type { HeaderRow } from "$lib/components/RequestForm.svelte";

    interface Props {
        rows: HeaderRow[];
        showAutoHeaders: boolean;
        autoHeaderCount: number;
        onUpdateRow: (idx: number, field: "key" | "value", val: string) => void;
        onRemoveRow: (idx: number) => void;
        onToggleRow: (idx: number) => void;
        onToggleAutoHeaders: () => void;
    }

    let {
        rows,
        showAutoHeaders,
        autoHeaderCount,
        onUpdateRow,
        onRemoveRow,
        onToggleRow,
        onToggleAutoHeaders,
    }: Props = $props();
</script>

<div class="flex items-center justify-between px-1 pb-1">
    <button
        class="btn btn-ghost btn-xs gap-1 text-xs font-normal opacity-60 hover:opacity-100"
        onclick={onToggleAutoHeaders}
    >
        {#if showAutoHeaders}
            <EyeOffIcon class="w-2.5 h-2.5"></EyeOffIcon>
            <span>Hide auto-generated headers</span>
        {:else}
            <EyeIcon class="w-2.5 h-2.5"></EyeIcon>
            <span>{autoHeaderCount} Hidden</span>
        {/if}
    </button>
</div>

<table class="table table-xs table-pin-rows table-pin-cols min-w-max">
    <thead>
        <tr>
            <th class="w-0"
                ><input
                    type="checkbox"
                    class="checkbox checkbox-xs invisible"
                /></th
            >
            <th class="w-auto text-xs">Key</th>
            <th class="w-auto text-xs">Value</th>
            <th class="w-0"></th>
        </tr>
    </thead>
    <tbody>
        {#each rows as row, i}
            {@const isAuto = row.auto}
            {@const isLast =
                i === rows.length - 1 && !row.key.trim() && !row.value.trim()}
            <tr
                class="group hover:bg-base-300 divide-x divide-base-content/10"
                class:opacity-50={isAuto}
                hidden={!showAutoHeaders && isAuto}
            >
                <td>
                    <input
                        type="checkbox"
                        class="checkbox checkbox-xs"
                        checked={row.enabled}
                        disabled={isLast}
                        hidden={isLast}
                        onchange={() => onToggleRow(i)}
                    />
                </td>
                <td>
                    <input
                        class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Key"
                        value={row.key}
                        oninput={(e) =>
                            onUpdateRow(
                                i,
                                "key",
                                (e.target as HTMLInputElement).value,
                            )}
                    />
                </td>
                <td>
                    <input
                        class="input input-ghost input-xs w-full font-mono p-0"
                        placeholder="Value"
                        value={row.value}
                        oninput={(e) =>
                            onUpdateRow(
                                i,
                                "value",
                                (e.target as HTMLInputElement).value,
                            )}
                    />
                </td>
                <td class="">
                    {#if !isLast}
                        <button
                            class="btn btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100"
                            onclick={() => onRemoveRow(i)}
                        >
                            ✕
                        </button>
                    {/if}
                </td>
            </tr>
        {/each}
    </tbody>
</table>
