<script lang="ts">
    import type { HttpMethod } from "$lib/types";
    import { methodTextColor } from "$lib/types";

    interface Props {
        method: HttpMethod;
    }

    let { method }: Props = $props();
</script>

<span class="font-mono font-bold {methodTextColor(method)}">{method}</span>
