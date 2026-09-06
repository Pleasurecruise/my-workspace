<script lang="ts">
	import type { Game } from "../../consumer";
	import { gameNames } from "../../dashboard";
	import GameNotesPanel from "./GameNotesPanel.svelte";
	import GachaPanel from "./GachaPanel.svelte";
	let { game }: { game: Game } = $props();
</script>

<section class="game-strip" class:mihoyo={game === "genshin" || game === "starRail" || game === "zzz"} aria-label={gameNames[game]}>
	<header><h2>{gameNames[game]}</h2><span>Official CN</span></header>
	<div class="content"><GameNotesPanel {game} /><GachaPanel {game} /></div>
</section>

<style>
	.game-strip { width: 100%; box-sizing: border-box; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.75rem 1rem 0; }
	h2 { margin: 0; font-size: 0.85rem; font-weight: 600; }
	header span { font-size: 0.65rem; color: var(--color-muted-foreground); }
	.content { display: grid; grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.4fr); }
	.mihoyo .content { height: 14rem; }
	.content :global(.game-panel) { min-width: 0; width: 100%; border: 0; border-radius: 0; background: transparent; box-shadow: none; padding: 0.5rem 0.75rem 0.75rem; overflow: auto; }
	.content :global(.game-panel + .game-panel) { border-left: 1px solid var(--color-border); }
	@container (max-width: 700px) { .content { grid-template-columns: 1fr; } .mihoyo .content { height: auto; } .content :global(.game-panel) { max-height: 14rem; } .content :global(.game-panel + .game-panel) { border-left: 0; border-top: 1px solid var(--color-border); } }
</style>
