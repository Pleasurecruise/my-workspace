<script lang="ts">
	import { HardDrive, RefreshCw } from "@lucide/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import type { CommandResponse, LocalStorageSample, StorageBreakdown } from "../consumer";

	let { storage, error }: { storage: LocalStorageSample | null; error: string | null } = $props();
	let breakdown = $state<StorageBreakdown | null>(null);
	let scanError = $state<string | null>(null);
	let scanning = $state(false);
	const amount = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });
	const labels = {
		system: "System files",
		applications: "Applications",
		documents: "Documents",
		development: "Development & builds",
		media: "Photos, video & audio",
		appData: "App data & caches",
		other: "Other scanned files",
	};


	async function scan(refresh: boolean) {
		if (scanning) return;
		scanning = true;
		scanError = null;
		try {
			const response = await invoke<CommandResponse<StorageBreakdown>>("read_storage", { refresh });
			if (response.status === "ready") breakdown = response.data;
			else scanError = response.message;
		} catch (error) {
			scanError = String(error);
		} finally {
			scanning = false;
		}
	}

	onMount(() => { void scan(false); });
</script>

<article class="storage-panel" aria-label="Device storage">
	<header><h2><HardDrive size={15} /> Device Storage</h2><button type="button" onclick={() => scan(true)} disabled={scanning} aria-label="Rescan storage categories" title="Rescan storage categories"><RefreshCw size={12} class={scanning ? "storage-scanning" : ""} /></button></header>
	{#if error !== null}
		<p class="message" role="alert">{error}</p>
	{:else if storage !== null}
		<p class="summary"><strong>{amount.format(storage.usedBytes / 1e9)} GB</strong><span>/ {amount.format(storage.totalBytes / 1e9)} GB</span></p>
		<div class="capacity" role="progressbar" aria-label="Startup disk used capacity" aria-valuenow={storage.usedPercent} aria-valuemin="0" aria-valuemax="100"><span style:width={`${storage.usedPercent}%`}></span></div>
		<p class="free">Startup disk · {amount.format((storage.totalBytes - storage.usedBytes) / 1e9)} GB free</p>
	{:else}
		<p class="message">Startup disk capacity unavailable</p>
	{/if}
	{#if breakdown !== null}
		<dl aria-label="Estimated storage categories">
			{#each breakdown.categories as item}
				<div><dt>{labels[item.category]}</dt><dd>{#if breakdown.incomplete && item.bytes === 0}Not measured{:else}{amount.format(item.bytes / 1e9)} GB{/if}</dd></div>
			{/each}
			{#if breakdown.unclassifiedBytes !== null}<div><dt>Unclassified / snapshots</dt><dd>{amount.format(breakdown.unclassifiedBytes / 1e9)} GB</dd></div>{/if}
		</dl>
		<p class="note" title="Estimates from readable files. APFS shared blocks, snapshots, permissions and unscanned folders can make category totals differ from disk usage.">{breakdown.incomplete ? "Partial scan · estimates" : "File scan · estimates"} · {new Date(breakdown.sampledAt * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</p>
	{/if}
	{#if scanning}<p class="note" role="status">Scanning storage…</p>{/if}
	{#if scanError !== null}<p class="message" role="alert">{scanError}</p>{/if}
</article>

<style>
	.storage-panel { width: 100%; min-width: 0; box-sizing: border-box; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header, h2 { display: flex; align-items: center; gap: 0.45rem; }
	header { justify-content: space-between; }
	h2 { margin: 0; color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.7rem; font-weight: 500; }
	button { display: grid; place-items: center; padding: 0.2rem; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	button:disabled { cursor: default; }
	button:focus-visible { outline: 2px solid var(--color-ring); outline-offset: 2px; }
	.summary { display: flex; align-items: baseline; gap: 0.4rem; margin: 0.55rem 0; font-family: var(--font-mono); }
	.summary strong { color: var(--color-foreground); font-size: 1rem; }
	.summary span, .free, .note { color: var(--color-muted-foreground); font-size: 0.6rem; }
	.capacity { height: 0.45rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.capacity span { display: block; height: 100%; background: var(--color-accent); }
	.free { margin: 0.35rem 0 0.65rem; }
	dl { margin: 0; font-size: 0.62rem; }
	dl div { display: flex; justify-content: space-between; gap: 0.5rem; padding: 0.2rem 0; }
	dt { color: var(--color-muted-foreground); }
	dd { margin: 0; white-space: nowrap; color: var(--color-foreground); font-family: var(--font-mono); }
	.note { margin: 0.4rem 0 0; }
	.message { color: var(--color-muted-foreground); font-size: 0.65rem; overflow-wrap: anywhere; }
	:global(.storage-scanning) { animation: spin 1s linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }
	@media (prefers-reduced-motion: reduce) { :global(.storage-scanning) { animation: none; } }
</style>
