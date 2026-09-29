<script lang="ts">
	import { LoaderCircle, Plug, Unplug } from "@lucide/svelte";
	import { Button } from "@my-workspace/ui";

	interface ConnectionStatusProps {
		name: string;
		connected: boolean;
		connecting?: boolean;
		disabled?: boolean;
		ontoggle: () => void;
	}

	let { name, connected, connecting = false, disabled = false, ontoggle }: ConnectionStatusProps = $props();
</script>

<div class="connection-status" class:connecting>
	<span role="status" class:online={connected}>{connecting ? "Connecting…" : connected ? "Online" : "Offline"}</span>
	<Button variant="ghost" size="icon-sm" disabled={disabled || connecting} aria-label={connected ? `Disconnect ${name}` : `Connect ${name}`} title={connected ? `Disconnect ${name}` : `Connect ${name}`} onclick={ontoggle}>
		{#if connecting}<LoaderCircle size={15} />{:else if connected}<Unplug size={15} />{:else}<Plug size={15} />{/if}
	</Button>
</div>

<style>
	.connection-status { display: flex; align-items: center; gap: 0.375rem; }
	span { color: var(--color-muted-foreground); font-family: var(--font-sans); font-size: 0.75rem; }
	span.online { color: var(--color-success); }
	.connecting :global(svg) { animation: connection-spin var(--duration-spinner) linear infinite; }
	@keyframes connection-spin { to { transform: rotate(360deg); } }
	@media (prefers-reduced-motion: reduce) { .connecting :global(svg) { animation: none; } }
</style>
