<script lang="ts">
	import { HardDrive } from "@lucide/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import type { CommandResponse, LocalStorageSample } from "../../consumer";

	let { storage, error }: { storage: LocalStorageSample | null; error: string | null } = $props();
	let settingsError = $state<string | null>(null);
	let opening = $state(false);
	const amount = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

	async function openSettings() {
		if (opening) return;
		opening = true;
		settingsError = null;
		const response = await invoke<CommandResponse<null>>("open_storage_settings");
		opening = false;
		if (response.status === "failed") settingsError = response.message;
	}
</script>

<article class="storage-panel" aria-label="Device storage">
	<header><h2><HardDrive size={15} /><span title="Device Storage">Device Storage</span></h2><button type="button" onclick={openSettings} disabled={opening}>{opening ? "Opening…" : "Open storage settings ↗"}</button></header>
	<div class="storage-content">
		<div class="storage-details">
			{#if error !== null}
				<p class="message" role="alert">{error}</p>
			{:else if storage !== null}
				<p class="summary"><strong>{amount.format(storage.usedBytes / 1e9)} GB</strong><span>/ {amount.format(storage.totalBytes / 1e9)} GB</span></p>
				<div class="capacity" role="progressbar" aria-label="Startup disk used capacity" aria-valuenow={storage.usedPercent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${storage.usedPercent}%`}></span></div>
				<p class="free">Startup disk · {amount.format((storage.totalBytes - storage.usedBytes) / 1e9)} GB free</p>
			{:else}
				<p class="message">Startup disk capacity unavailable</p>
			{/if}
			<p class="note">View category usage in system storage settings.</p>
			{#if settingsError !== null}<p class="message" role="alert">{settingsError}</p>{/if}
		</div>
	</div>
</article>

<style>
	.storage-panel { display: flex; flex-direction: column; min-height: 0; contain: size; overflow: hidden; width: 100%; min-width: 0; box-sizing: border-box; padding: 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.storage-content { display: flex; flex-direction: column; flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; margin-top: 0.45rem; }
	.storage-details { flex-shrink: 0; margin-block: auto; }
	header { flex-shrink: 0; flex-wrap: nowrap; }
	header, h2 { display: flex; align-items: center; gap: 0.45rem; }
	header { justify-content: space-between; }
	header button { flex-shrink: 0; white-space: nowrap; margin-left: auto; padding: 0; font-family: var(--font-mono); font-size: 0.58rem; }
	header button:hover:not(:disabled) { color: var(--color-success); }

	h2 { min-width: 0; margin: 0; color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.7rem; font-weight: 500; }
	h2 :global(svg) { flex-shrink: 0; }
	h2 span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	button { display: grid; place-items: center; padding: 0.2rem; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	button:disabled { cursor: default; }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.summary { display: flex; align-items: baseline; gap: 0.4rem; margin: 0.55rem 0; font-family: var(--font-mono); }
	.summary strong { color: var(--color-foreground); font-size: 1rem; }
	.summary span, .free, .note { color: var(--color-muted-foreground); font-size: 0.6rem; }
	.capacity { height: 0.45rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.capacity span { display: block; height: 100%; background: var(--color-accent); }
	.free { margin: 0.35rem 0 0.65rem; }
	.note { margin: 0.4rem 0 0; }
	.message { color: var(--color-muted-foreground); font-size: 0.65rem; overflow-wrap: anywhere; }
</style>
