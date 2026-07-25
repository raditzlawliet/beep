<script lang="ts">
    import type { Auth, HeaderField } from "$lib/types";

    interface Props {
        headers: HeaderField[];
        onUpdate: (headers: HeaderField[]) => void;
    }

    let { headers, onUpdate }: Props = $props();

    const auth = $derived<Auth>(detectAuth(headers));

    function detectAuth(hdrs: HeaderField[]): Auth {
        for (const h of hdrs) {
            if (!h.enabled || h.auto) continue;
            if (h.key.toLowerCase() !== "authorization") continue;
            const val = h.value.trim();

            if (val.startsWith("Bearer")) {
                const token = val.startsWith("Bearer ") ? val.slice(7).trim() : "";
                return { type: "Bearer", token };
            }
            if (val.startsWith("Basic")) {
                const creds = val.startsWith("Basic ") ? val.slice(6).trim() : "";
                try {
                    const decoded = atob(creds);
                    const colon = decoded.indexOf(":");
                    if (colon >= 0) {
                        return { type: "Basic", username: decoded.slice(0, colon), password: decoded.slice(colon + 1) };
                    }
                } catch {}
                const colon = creds.indexOf(":");
                if (colon >= 0) {
                    return { type: "Basic", username: creds.slice(0, colon), password: creds.slice(colon + 1) };
                }
                const space = creds.indexOf(" ");
                if (space >= 0) {
                    return { type: "Basic", username: creds.slice(0, space), password: creds.slice(space + 1) };
                }
                return { type: "Basic", username: creds, password: "" };
            }

            // Any other value (including empty) will be Custom
            return { type: "Custom", value: val };
        }
        return { type: "None" };
    }

    /// Mutate the existing Authorization header in-place, or add/remove as needed.
    function applyAuth(newAuth: Auth) {
        const idx = headers.findIndex((h) => !h.auto && h.key.toLowerCase() === "authorization");

        // Remove auth
        if (newAuth.type === "None") {
            if (idx < 0) return;
            const copy = [...headers];
            copy.splice(idx, 1);
            onUpdate(copy);
            return;
        }

        // Build new header value
        let value: string;
        if (newAuth.type === "Bearer") {
            value = `Bearer ${newAuth.token}`;
        } else if (newAuth.type === "Basic") {
            value = `Basic ${newAuth.username}:${newAuth.password}`;
        } else {
            value = newAuth.value;
        }

        if (idx >= 0) {
            // Mutate existing header in-place
            const copy = [...headers];
            copy[idx] = { ...copy[idx], value, enabled: true };
            onUpdate(copy);
        } else {
            onUpdate([...headers, { key: "Authorization", value, enabled: true, auto: false }]);
        }
    }

    // Local state for select binding. Synced from headers via effect.
    let selectedType = $state("None");

    $effect(() => {
        // Read headers (reactive prop) to establish dependency
        const h = headers;
        const a = detectAuth(h);
        selectedType = a.type;
    });

    function handleTypeChange() {
        if (selectedType === "None") applyAuth({ type: "None" });
        else if (selectedType === "Basic") applyAuth({ type: "Basic", username: "", password: "" });
        else if (selectedType === "Bearer") applyAuth({ type: "Bearer", token: "" });
        else applyAuth({ type: "Custom", value: "" });
    }

    function handleFieldUpdate(newAuth: Auth) {
        applyAuth(newAuth);
    }
</script>

<div class="flex gap-6 flex-1 min-h-0 px-2">
    <div class="w-[30%] min-w-44 flex flex-col gap-2">
        <div class="text-xs font-semibold opacity-70 uppercase tracking-wide">Auth Type</div>
        <select class="select select-bordered select-sm w-full" bind:value={selectedType} onchange={handleTypeChange}>
            <option value="None">No Auth</option>
            <option value="Basic">Basic Auth</option>
            <option value="Bearer">Bearer Token</option>
            <option value="Custom">Custom</option>
        </select>
        <p class="text-xs opacity-50 mt-1 leading-relaxed">
            {#if auth.type === "None"}
                No authentication will be sent with this request.
            {:else if auth.type === "Basic"}
                Sends a username and password encoded in Base64 via the
                Authorization header.
            {:else if auth.type === "Bearer"}
                Sends a Bearer token in the Authorization header.
            {:else}
                Sends <code class="text-xs bg-base-300 px-0.5 rounded">Authorization</code> header with custom scheme.
            {/if}
        </p>
    </div>

    <div class="flex-1 flex flex-col gap-2">
        {#if auth.type === "None"}
            <div class="flex flex-col items-center justify-center h-full text-sm gap-1">
                <div class="text-base font-semibold opacity-70">No Auth</div>
                <div class="opacity-50">This request does not use any authorization.</div>
            </div>
        {:else if auth.type === "Basic"}
            <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs opacity-70">Username</span></div>
                <input class="input input-bordered input-sm w-full" placeholder="Username"
                    value={auth.username}
                    oninput={(e) => handleFieldUpdate({ ...auth, username: (e.target as HTMLInputElement).value } as Auth)} />
            </label>
            <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs opacity-70">Password</span></div>
                <input class="input input-bordered input-sm w-full" type="password" placeholder="Password"
                    value={auth.password}
                    oninput={(e) => handleFieldUpdate({ ...auth, password: (e.target as HTMLInputElement).value } as Auth)} />
            </label>
        {:else if auth.type === "Bearer"}
            <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs opacity-70">Token</span></div>
                <input class="input input-bordered input-sm w-full" placeholder="eyJ..."
                    value={auth.token}
                    oninput={(e) => handleFieldUpdate({ type: "Bearer", token: (e.target as HTMLInputElement).value })} />
            </label>
        {:else}
            <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs opacity-70">Authorization Value</span></div>
                <input class="input input-bordered input-sm w-full font-mono"
                    placeholder="CustomScheme credentials"
                    value={auth.value}
                    oninput={(e) => handleFieldUpdate({ type: "Custom", value: (e.target as HTMLInputElement).value })} />
            </label>
        {/if}
    </div>
</div>
