<script lang="ts">
    import type { Auth } from "$lib/types";

    interface Props {
        auth: Auth;
        onUpdate: (auth: Auth) => void;
    }

    let { auth, onUpdate }: Props = $props();

    function handleTypeChange(e: Event) {
        const t = (e.target as HTMLSelectElement).value;
        if (t === "None") onUpdate({ type: "None" });
        else if (t === "Basic")
            onUpdate({ type: "Basic", username: "", password: "" });
        else onUpdate({ type: "Bearer", token: "" });
    }
</script>

<div class="flex gap-6 flex-1 min-h-0 px-2">
    <!-- Left panel: Auth Type selector -->
    <div class="w-[30%] min-w-44 flex flex-col gap-2">
        <div
            class="text-xs font-semibold opacity-70 uppercase tracking-wide"
        >
            Auth Type
        </div>
        <select
            class="select select-bordered select-sm w-full"
            value={auth.type}
            onchange={handleTypeChange}
        >
            <option value="None">No Auth</option>
            <option value="Basic">Basic Auth</option>
            <option value="Bearer">Bearer Token</option>
        </select>
        <p class="text-xs opacity-50 mt-1 leading-relaxed">
            {#if auth.type === "None"}
                No authentication will be sent with this request.
            {:else if auth.type === "Basic"}
                Sends a username and password encoded in Base64
                via the Authorization header.
            {:else}
                Sends a Bearer token in the Authorization header.
            {/if}
        </p>
    </div>

    <!-- Right panel: Auth fields -->
    <div class="flex-1 flex flex-col gap-2">
        {#if auth.type === "None"}
            <div
                class="flex flex-col items-center justify-center h-full text-sm gap-1"
            >
                <div class="text-base font-semibold opacity-70">
                    No Auth
                </div>
                <div class="opacity-50">
                    This request does not use any authorization.
                </div>
            </div>
        {:else if auth.type === "Basic"}
            <label class="form-control w-full">
                <div class="label py-1">
                    <span class="label-text text-xs opacity-70"
                        >Username</span
                    >
                </div>
                <input
                    class="input input-bordered input-sm w-full"
                    placeholder="Username"
                    value={auth.username}
                    oninput={(e) =>
                        onUpdate({
                            ...auth,
                            username: (e.target as HTMLInputElement).value,
                        } as Auth)}
                />
            </label>
            <label class="form-control w-full">
                <div class="label py-1">
                    <span class="label-text text-xs opacity-70"
                        >Password</span
                    >
                </div>
                <input
                    class="input input-bordered input-sm w-full"
                    type="password"
                    placeholder="Password"
                    value={auth.password}
                    oninput={(e) =>
                        onUpdate({
                            ...auth,
                            password: (e.target as HTMLInputElement).value,
                        } as Auth)}
                />
            </label>
        {:else if auth.type === "Bearer"}
            <label class="form-control w-full">
                <div class="label py-1">
                    <span class="label-text text-xs opacity-70">Token</span>
                </div>
                <input
                    class="input input-bordered input-sm w-full"
                    placeholder="eyJ..."
                    value={auth.token}
                    oninput={(e) =>
                        onUpdate({
                            type: "Bearer",
                            token: (e.target as HTMLInputElement).value,
                        })}
                />
            </label>
        {/if}
    </div>
</div>
