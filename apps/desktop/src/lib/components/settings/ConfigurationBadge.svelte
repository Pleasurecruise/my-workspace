<script lang="ts">
	import { CircleCheck } from "@lucide/svelte";
	let { label = "Configured" }: { label?: string } = $props();
</script>

<span class="configuration-badge"><CircleCheck size={14} /> {label}</span>

<style>
	.configuration-badge { display: inline-flex; width: fit-content; align-items: center; gap: 0.3rem; padding: 0.25rem 0.45rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-success) 8%, transparent); color: var(--color-success); font-size: 0.62rem; white-space: nowrap; }
</style>
