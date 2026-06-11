<script lang="ts">
  interface Props {
    class?: string;
  }

  let { class: className = 'h-4 w-4' }: Props = $props();
</script>

<svg
  class={className}
  xmlns="http://w3.org"
  width="24"
  height="24"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
>
  <rect x="4" y="8" width="12" height="12" rx="1" />
  <path d="M8 8V4a1 1 0 0 1 1-1h11a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1h-4" />
</svg>
