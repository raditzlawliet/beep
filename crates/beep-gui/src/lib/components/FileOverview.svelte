<script lang="ts">
    import type { ParsedRequest, ParsedFileVariable, HttpMethod } from "$lib/types";
    import MethodBadge from "$lib/components/MethodBadge.svelte";
    import AddRowButton from "$lib/components/buttons/AddRowButton.svelte";
    import DeleteRowButton from "$lib/components/buttons/DeleteRowButton.svelte";
    import { ArrowUpIcon, ArrowUpRightIcon } from "@lucide/svelte";

    interface Props {
        requests: ParsedRequest[];
        variables: ParsedFileVariable[];
        activeRequestIdx: number;
        onNavigateToRequest: (idx: number) => void;
        onVariablesUpdate: (vars: ParsedFileVariable[]) => void;
    }

    let {
        requests,
        variables,
        activeRequestIdx,
        onNavigateToRequest,
        onVariablesUpdate,
    }: Props = $props();

    type SubTab = "requests" | "variables";
    let subTab = $state<SubTab>("requests");

    function addVariable() {
        onVariablesUpdate([...variables, { key: "", value: "" }]);
    }

    function removeVariable(idx: number) {
        onVariablesUpdate(variables.filter((_, i) => i !== idx));
    }

    function updateKey(idx: number, key: string) {
        const vars = [...variables];
        vars[idx] = { ...vars[idx], key };
        onVariablesUpdate(vars);
    }

    function updateValue(idx: number, value: string) {
        const vars = [...variables];
        vars[idx] = { ...vars[idx], value };
        onVariablesUpdate(vars);
    }
</script>

<div class="card rounded-none bg-base-100 flex-1 min-h-0">
    <div class="card-body p-0 flex flex-col min-h-0">
    <div role="tablist" class="tabs tabs-bordered tabs-xs px-1 pt-1">
        <button
            role="tab"
            class="tab gap-1.5 {subTab === 'requests' ? 'tab-active' : ''}"
            onclick={() => (subTab = 'requests')}
        >
            Requests
            <span class="text-xs opacity-50">({requests.length})</span>
        </button>
        <button
            role="tab"
            class="tab gap-1.5 {subTab === 'variables' ? 'tab-active' : ''}"
            onclick={() => (subTab = 'variables')}
        >
            Variables
            <span class="text-xs opacity-50">({variables.length})</span>
        </button>
    </div>
    <div class="border-b border-b-base-content/10"></div>

    {#if subTab === "requests"}
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col">
            {#if requests.length === 0}
                <div class="text-xs opacity-50 p-4 text-center">No requests found</div>
            {:else}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="flex flex-col">
                    {#each requests as req, i}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div
                            class="flex items-center gap-1.5 px-1 py-1.5 rounded
                                hover:bg-base-200 text-xs {i === activeRequestIdx ? 'bg-base-200' : ''}"
                        >
                            <span class="opacity-40 w-min text-right ps-1">#{i + 1}</span>
                            <MethodBadge method={req.method as HttpMethod} />
                            <span class="truncate flex-1 font-mono">{req.url || "(no URL)"}</span>
                            <span class="text-xs opacity-50 truncate max-w-32">{req.title || ""}</span>
                            <button
                                class="btn btn-ghost btn-xs"
                                onclick={() => onNavigateToRequest(i)}
                                title="Edit request"
                            >
                                <ArrowUpRightIcon class="h-3.5 w-3.5" />
                            </button>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {:else}
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col">
            <table class="table table-xs table-pin-rows table-pin-cols min-w-max">
                <thead>
                    <tr>
                        <th class="w-auto text-xs">Key</th>
                        <th class="w-auto text-xs">Value</th>
                        <th class="w-0"></th>
                    </tr>
                </thead>
                <tbody>
                    {#each variables as v, i (i)}
                        <tr class="group hover:bg-base-300 divide-x divide-base-content/10">
                            <td>
                                <input class="input input-ghost input-xs w-full font-mono p-0"
                                    placeholder="Key" value={v.key}
                                    oninput={(e) => updateKey(i, (e.target as HTMLInputElement).value)} />
                            </td>
                            <td>
                                <input class="input input-ghost input-xs w-full font-mono p-0"
                                    placeholder="Value" value={v.value}
                                    oninput={(e) => updateValue(i, (e.target as HTMLInputElement).value)} />
                            </td>
                            <td>
                                <DeleteRowButton onclick={() => removeVariable(i)} />
                            </td>
                        </tr>
                    {/each}
                    <tr>
                        <td class="p-0">
                            <AddRowButton onclick={addVariable} text="Add variable" />
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    {/if}
    </div>
</div>
