<script lang="ts">
    import ScriptTab from "./ScriptTab.svelte";

    interface Props {
        value: string | null;
        external: boolean;
        onchange?: (value: string | null, isExternal: boolean) => void;
        basePath?: string | null;
    }

    let { value, external = false, onchange, basePath = null }: Props = $props();
</script>

<ScriptTab
    {value}
    {external}
    {onchange}
    {basePath}
    description={'Script will be run <span class="text-accent">after request is sent</span>.'}
/>
