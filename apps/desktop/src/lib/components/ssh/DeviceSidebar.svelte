<script lang="ts">
	import { RefreshCw, Terminal } from "@lucide/svelte";
	import type { createSshSession } from "./session.svelte";
	import type { SshDevice } from "../../consumer";
	let { session, compact, active, onopen }: {
		session: ReturnType<typeof createSshSession>;
		compact: boolean;
		active: boolean;
		onopen: (device: SshDevice) => void;
	} = $props();
</script>

<section class="devices" class:compact aria-label="Tailscale devices">
	<header><span>Tailscale</span><button type="button" disabled={session.loading} aria-label="Refresh Tailscale devices" title="Refresh Tailscale devices" onclick={() => void session.refresh()}><RefreshCw size={12} /></button></header>
	{#if session.error}<p class="error" role="status" title={session.error}>{compact ? "!" : session.error}</p>{/if}
	{#if session.devices.length === 0 && session.error === null}<p>{compact ? "…" : session.loading ? "Discovering devices…" : "No devices tagged tag:server in this tailnet."}</p>{/if}
	<div class="device-list">
		{#each session.devices as device (device.id)}
			{@const status = device.online === null ? "Status unknown" : device.online ? "Online" : "Offline"}
			<button type="button" class:selected={active && session.selectedId === device.id} aria-current={active && session.selectedId === device.id ? "page" : "false"} aria-label={`${device.name}, ${status}, open SSH terminal`} title={`${device.name} · ${status}\n${device.dnsName || device.address}`} onclick={() => onopen(device)}>
				<span class="status-dot" class:online={device.online === true} class:offline={device.online === false} aria-hidden="true"></span>
				{#if compact}<Terminal size={14} />{:else}<span class="device-name">{device.name || device.address}<small>{device.os || "Device"} · {status}</small></span><Terminal size={12} />{/if}
			</button>
		{/each}
	</div>
</section>

<style>
	.devices { padding: 1rem 0.75rem 0.5rem; min-height: 0; }
	header { display: flex; align-items: center; justify-content: space-between; padding: 0 0.75rem 0.4rem; color: var(--color-muted-foreground); font-size: 0.65rem; letter-spacing: 0.05em; }
	button { border: 0; background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	header button { display: grid; place-items: center; padding: 4px; border-radius: var(--radius-sm); }
	button:disabled { opacity: 0.5; cursor: wait; }
	.device-list { display: grid; gap: 2px; max-height: 35dvh; overflow-y: auto; }
	.device-list button { display: flex; align-items: center; gap: 0.6rem; width: 100%; min-height: 2.6rem; padding: 0.45rem 0.75rem; text-align: left; border-radius: var(--radius-md); }
	button:hover, button.selected { background: color-mix(in srgb, var(--color-accent) 10%, transparent); color: var(--color-accent); }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: -2px; }
	.device-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.78rem; }
	small { display: block; margin-top: 3px; color: var(--color-muted-foreground); font-size: 0.6rem; }
	.status-dot { flex: 0 0 6px; height: 6px; border-radius: var(--radius-full); background: var(--color-warning); }
	.status-dot.online { background: var(--color-success); }
	.status-dot.offline { background: var(--color-muted-foreground); opacity: 0.55; }
	p { margin: 0.25rem 0.75rem; font-size: 0.65rem; line-height: 1.5; color: var(--color-muted-foreground); }
	.error { color: var(--color-warning); overflow-wrap: anywhere; }
	.compact { padding-inline: 0.4rem; }
	.compact header { justify-content: center; padding-inline: 0; }
	.compact header > span { display: none; }
	.compact .device-list button { justify-content: center; gap: 5px; padding-inline: 4px; }
</style>
