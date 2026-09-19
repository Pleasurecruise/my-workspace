<script lang="ts">
	import { RefreshCw } from "@lucide/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import { onMount } from "svelte";
	import type { CommandResponse, SteamGames } from "../../consumer";
	import "./games.css";
	let games = $state<SteamGames | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let refreshing = $state(false);
	let active = false;
	let revision = 0;
	function formatHours(minutes: number | null) { return minutes === null ? "Unavailable" : `${(minutes / 60).toLocaleString(undefined, { maximumFractionDigits: 1 })} h`; }
	function status(state: number | null) {
		switch (state) {
			case 0: return "Offline"; case 1: return "Online"; case 2: return "Busy";
			case 3: return "Away"; case 4: return "Snooze"; case 5: return "Looking to trade";
			case 6: return "Looking to play"; default: return "Status unavailable";
		}
	}
	async function refresh() {
		if (refreshing) return;
		refreshing = true;
		const current = ++revision;
		const result = await invoke<CommandResponse<SteamGames>>("read_steam_games");
		if (!active) return;
		refreshing = false;
		if (current !== revision) return;
		loading = false;
		if (result.status === "ready") { games = result.data; error = null; }
		else error = result.message;
	}
	onMount(() => {
		active = true;
		const subscription = listen<CommandResponse<SteamGames>>("steam-games-updated", ({ payload }) => {
			if (!active) return;
			revision += 1; loading = false;
			if (payload.status === "ready") { games = payload.data; error = null; }
			else error = payload.message;
		});
		void refresh();
		return () => { active = false; void subscription.then((unlisten) => unlisten()); };
	});
</script>

<section class="game-panel steam-panel" aria-label="Steam games">
	<header><h2>Steam</h2><div class="header-actions">{#if games !== null}<span class="muted">Updated {new Date(games.sampledAt * 1000).toLocaleTimeString()}</span>{/if}<button class="refresh" class:spinning={refreshing} type="button" disabled={refreshing} onclick={refresh} title={refreshing ? "Refreshing Steam…" : "Refresh Steam"} aria-label={refreshing ? "Refreshing Steam" : "Refresh Steam"}><RefreshCw size={14} /></button></div></header>
	{#if loading}<p class="muted" role="status">Loading Steam…</p>{/if}
	{#if error !== null}<p class="error" role="alert">{error}</p>{/if}
	{#if games !== null}
		<div class="content"><div class="overview"><div class="profile"><strong>{games.name}</strong><span class="muted">{status(games.state)}</span></div>
		{#if games.playing !== null}<p class="playing">Playing now · <strong>{games.playing}</strong></p>{/if}
		<div class="stats">
			<div><span>Last 2 weeks</span><strong>{formatHours(games.recentMinutes)}</strong><small>{games.recentCount === null ? "Activity unavailable" : `${games.recentCount} games played`}</small></div>
			<div><span>Library playtime</span><strong>{formatHours(games.totalMinutes)}</strong><small>{games.playedGames === null ? "Playtime unavailable" : `${games.playedGames} games played`}</small></div>
			<div><span>Library</span><strong>{games.ownedGames === null ? "Unavailable" : games.ownedGames.toLocaleString()}</strong><small>Including played free games</small></div>
		</div>
		</div><div class="lists">
			{#each [{ title: "Most played", items: games.mostPlayed, recent: false, available: games.ownedGames }, { title: "Recent activity", items: games.recent, recent: true, available: games.recentCount }] as list (list.title)}
				<section aria-label={list.title}><h3>{list.title}<span class="muted">{list.recent ? "2 weeks" : "All time"}</span></h3>
					{#if list.available === null}<p class="muted">Game details unavailable. Check your Steam profile privacy settings.</p>
					{:else if list.items.length === 0}<p class="muted">{list.recent ? "No games played in the last 2 weeks." : "No recorded playtime yet."}</p>
					{:else}<ol>{#each list.items as item (item.appId)}<li><button class="game" type="button" onclick={() => void openUrl(`https://store.steampowered.com/app/${item.appId}/`)}><span title={item.name}>{item.name}</span><small title={list.recent ? `${formatHours(item.playtimeForever)} total` : "Total playtime"}>{list.recent ? formatHours(item.playtime2weeks) : formatHours(item.playtimeForever)}</small></button></li>{/each}</ol>{/if}
				</section>
			{/each}
		</div>
		</div>
	{/if}
</section>

<style>
	.game-panel .refresh { display: grid; place-items: center; width: 1.6rem; height: 1.6rem; padding: 0; border: 0; background: transparent; color: var(--color-muted-foreground); }
	.refresh:hover { background: var(--color-muted); color: var(--color-foreground); }
	.refresh:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.spinning :global(svg) { animation: spin var(--duration-spinner) linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }
	@media (prefers-reduced-motion: reduce) { .spinning :global(svg) { animation: none; } }
	.steam-panel { container-type: inline-size; padding: 0; }
	.game-panel > header { padding: 0.75rem 1.25rem 0; margin-bottom: 0; }
	.steam-panel > p { margin: 0.75rem 1.25rem; }
	.game-panel h2 { font-size: 0.85rem; font-weight: 600; }
	.content { display: grid; grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.4fr); }
	.overview { min-width: 0; padding: 0.75rem 1.25rem; }
	.header-actions { display: flex; align-items: center; gap: 0.75rem; }
	.profile { display: flex; align-items: baseline; flex-wrap: wrap; gap: 0.5rem; }
	.profile strong { font-size: 1rem; overflow-wrap: anywhere; }
	.playing { padding: 0.4rem 0.5rem; border-radius: var(--radius-md); background: var(--color-muted); color: var(--color-accent); }
	.stats { display: grid; gap: 0.5rem; padding: 0.75rem 0 0; }
	.stats > div { display: grid; grid-template-columns: minmax(0, 1fr) max-content; align-items: baseline; gap: 0.1rem 0.75rem; }
	.stats small { grid-column: 1 / -1; }
	.stats span, small { font-size: 0.7rem; color: var(--color-muted-foreground); }
	.stats strong { font-family: var(--font-mono); font-size: 1.1rem; text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }
	.lists { min-width: 0; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1.25rem; padding: 0.75rem 1.25rem; border-left: 1px solid var(--color-border); }
	.lists > section { min-width: 0; }
	h3 { display: flex; align-items: baseline; justify-content: space-between; gap: 0.5rem; margin: 0 0 0.25rem; font-size: 0.8rem; }
	ol { padding: 0; margin: 0; list-style: none; }
	li + li { border-top: 1px solid var(--color-border); }
	.game-panel .game { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 0.3rem 0.6rem; width: 100%; border: 0; padding: 0.4rem 0; text-align: left; }
	.game span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.game small { white-space: nowrap; }
	.game:hover span { color: var(--color-accent); }
	@container (max-width: 700px) { .content { grid-template-columns: 1fr; } .lists { border-left: 0; border-top: 1px solid var(--color-border); } }
	@container (max-width: 480px) { .lists { grid-template-columns: 1fr; } .stats strong { font-size: 1.1rem; } }
</style>
