<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { Select } from "@my-workspace/ui";
	import { RefreshCw } from "@lucide/svelte";
	import { onDestroy, onMount } from "svelte";
	import type { CommandResponse, GachaArchive, Game } from "../../consumer";
	import { gameNames } from "../../dashboard";
	import "./games.css";
	import StarRailGachaReport from "./StarRailGachaReport.svelte";
	let { game }: { game: Game } = $props();
	let archive = $state<GachaArchive | null>(null);
	let error = $state<string | null>(null);
	let message = $state<string | null>(null);
	let busy = $state(false);
	let selectedUid = $state("");
	let generation = 0;
	onDestroy(() => { generation += 1; });
	async function read(uid: string | null) {
		const current = ++generation;
		busy = true;
		const result = await invoke<CommandResponse<GachaArchive>>("read_gacha_archive", { game, uid });
		if (current !== generation) return;
		busy = false;
		if (result.status === "ready") { archive = result.data; selectedUid = result.data.uid === null ? "" : result.data.uid; error = null; }
		else error = result.message;
	}
	async function sync() {
		if (busy) return;
		const current = ++generation;
		busy = true; message = null; error = null;
		const result = await invoke<CommandResponse<GachaArchive>>("sync_gacha_archive", { game });
		if (current !== generation) return;
		busy = false;
		if (result.status === "ready") {
			archive = result.data; selectedUid = result.data.uid === null ? "" : result.data.uid;
			message = result.data.official === null ? `${result.data.added} new pulls archived.` : `${result.data.added} new 5★ records saved.`;
		} else error = result.message;
	}
	onMount(() => { void read(null); });
</script>

<section class="game-panel compact" aria-label={`${gameNames[game]} pull analysis`}>
	<header><div class="heading"><h3>Pull archive</h3><button class="sync" class:spinning={busy} disabled={busy} onclick={sync} aria-label="Sync history" title="Sync history"><RefreshCw size={13} /></button></div><span class="muted" title={archive === null || archive.uid === null ? "Local history" : `Archive · ${archive.uid}`}>{archive === null ? "Local history" : archive.official === null ? `${archive.total} archived pulls` : `${archive.official.pools.reduce((sum, pool) => sum + pool.total, 0)} reported pulls`}</span></header>
	{#if error !== null}<p class="error" role="alert">{error}</p>{/if}
	{#if message !== null}<p class="muted" role="status">{message}</p>{/if}
	{#if archive !== null}
		{#if archive.accounts.length > 1}<div class="archive-choice"><Select label="Archived game account" disabled={busy} value={selectedUid} options={archive.accounts.map((account) => ({ value: account.uid, label: `${account.name} · ${account.uid}` }))} onchange={read} /></div>{/if}
		{#if archive.official !== null}<StarRailGachaReport report={archive.official} />{/if}
		{#if archive.official === null && archive.total === 0}<p class="empty muted">No archived pulls yet. Sync once to build your local history.</p>{/if}
		{#if archive.official === null && archive.pools.length > 0}
			<div class="pools">{#each archive.pools as pool (pool.id)}
				<div class="pool"><h4>{pool.name}</h4><div class="distribution">
					<svg viewBox="0 0 100 100" role="img" aria-label={`${pool.name}: ${pool.total} pulls; ${pool.rarities.map((count, index) => `${index + 1} star: ${count}`).join(", ")}`}>
						<circle class="track" cx="50" cy="50" r="39" />
						{#each pool.rarities as count, index}{#if count > 0}<circle class={`rarity rarity-${index + 1}`} cx="50" cy="50" r="39" pathLength="100" stroke-dasharray={`${count / pool.total * 100} 100`} stroke-dashoffset={-pool.rarities.slice(0, index).reduce((sum, value) => sum + value, 0) / pool.total * 100} />{/if}{/each}
						<text x="50" y="49" class="total">{pool.total}</text><text x="50" y="63" class="caption">pulls</text>
					</svg>
					<div class="legend">{#each pool.rarities as count, index}{#if count > 0}<span><i class={`rarity-${index + 1}`}></i>{index + 1}★ <b>{count}</b></span>{/if}{/each}</div>
				</div><p class="muted">Since top rarity <strong>{pool.highRarity === 0 ? "≥ " : ""}{pool.sinceHighRarity}</strong></p><p class="muted">Avg. interval <strong>{pool.averageInterval === null ? "—" : pool.averageInterval.toFixed(1)}</strong></p></div>
			{/each}</div>
		{/if}
		{#if archive.recent.length > 0}
			<details><summary>Recent pulls & archive details</summary><p class="muted">Sync is manual. Earlier records stay saved. Intervals describe this archive, not guaranteed pity.</p>{#if archive.syncedAt !== null}<p class="muted">Last synced {new Date(archive.syncedAt * 1000).toLocaleString()}</p>{/if}<div class="table-scroll"><table>
				<thead><tr><th>Item</th><th>Rarity</th><th>Time</th></tr></thead>
				<tbody>{#each archive.recent as pull (pull.id)}
					<tr><td>{pull.name}{#if pull.isFree === true}<span class="muted"> · Free</span>{/if}{#if pull.isNew === true}<span class="muted"> · New</span>{/if}</td><td>{pull.rarity}★</td><td>{pull.time}</td></tr>
				{/each}</tbody>
			</table></div></details>
		{/if}
	{/if}
	{#if archive === null || (archive.official === null && archive.recent.length === 0)}<details class="archive-notes"><summary>About this archive</summary><p class="muted">Sync is manual. Earlier records stay saved. Intervals describe this archive, not guaranteed pity.</p>{#if archive !== null && archive.syncedAt !== null}<p class="muted">Last synced {new Date(archive.syncedAt * 1000).toLocaleString()}</p>{/if}</details>{/if}
</section>

<style>
	h3 { margin: 0; font-size: 0.75rem; font-weight: 500; color: var(--color-muted-foreground); }
	.heading { display: flex; align-items: center; gap: 0.4rem; }
	.game-panel .sync { display: grid; place-items: center; width: 1.6rem; height: 1.6rem; padding: 0; border: 0; background: transparent; color: var(--color-muted-foreground); }
	.game-panel .sync:hover { color: var(--color-foreground); background: var(--color-muted); }
	.spinning :global(svg) { animation: spin var(--duration-spinner) linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }
	@media (prefers-reduced-motion: reduce) { .spinning :global(svg) { animation: none; } }
	.archive-choice { margin-top: 0.75rem; max-width: 20rem; }
	.pools { display: grid; grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr)); gap: 0.75rem; margin: 0.65rem 0; }
	h4 { margin: 0 0 0.4rem; font-size: 0.7rem; font-weight: 500; }
	.distribution { display: flex; align-items: center; gap: 0.5rem; }
	svg { width: 5.5rem; flex-shrink: 0; overflow: visible; }
	circle { fill: none; stroke-width: 9; }
	.track { stroke: var(--color-muted); }
	.rarity { stroke: currentColor; transform: rotate(-90deg); transform-origin: 50% 50%; }
	.rarity-1, .rarity-2, .rarity-3 { color: var(--color-muted-foreground); }
	.rarity-4 { color: var(--color-accent); }
	.rarity-5 { color: var(--color-warning); }
	.rarity-6 { color: var(--color-success); }
	text { text-anchor: middle; fill: var(--color-foreground); font-family: var(--font-mono); }
	.total { font-size: 17px; font-weight: 600; }
	.caption { font-size: 9px; fill: var(--color-muted-foreground); }
	.legend { display: grid; gap: 0.35rem; font-size: 0.65rem; }
	.legend span { display: flex; gap: 0.3rem; align-items: center; white-space: nowrap; color: var(--color-muted-foreground); }
	.legend b { color: var(--color-foreground); font-weight: 500; }
	.legend i { width: 0.35rem; height: 0.35rem; border-radius: 50%; background: currentColor; }
	.pool p { display: flex; justify-content: space-between; gap: 0.5rem; }
	.pool strong { color: var(--color-foreground); font-weight: 500; }
	.compact .pools { grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr)); gap: 0.5rem 0.75rem; margin: 0.5rem 0; }
	.compact svg { width: 3.5rem; }
	.compact .legend { gap: 0.15rem; font-size: 0.6rem; }
	.compact .pool p { margin: 0.2rem 0; font-size: 0.6rem; }
	.compact h4 { margin-bottom: 0.25rem; }
	.archive-notes { display: inline-block; }
	.empty { padding: 0.75rem 0; }
</style>
