<script lang="ts">
	import type { Snippet } from "svelte";
	import { Clock, Type } from "@lucide/svelte";
	import type { ReadingStats } from "../../consumer";

	let { title, stats, actions }: { title: string; stats: ReadingStats; actions: Snippet } = $props();
</script>

<header class="page-header"><div>
	<h1>{title}</h1>
	{#if stats.wordCount > 0}
		<div class="stats">
			<span><Type size={13} />{stats.wordCount}</span><i>·</i><span><Clock size={13} />{stats.readingMinutes} min</span>
		</div>
	{/if}
</div>
	{@render actions()}
</header>

<style>
	header.page-header { min-width: 0; display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; column-gap: 1rem; }
	header > div { display: contents; }
	header > :global(:last-child) { grid-column: 2; grid-row: 1; align-self: center; }
	h1 { margin: 0; grid-column: 1; grid-row: 1; }
	.stats { grid-column: 1 / -1; grid-row: 2; display: flex; align-items: center; gap: 0.4rem; margin-top: 0.5rem; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.stats span { display: inline-flex; align-items: center; gap: 0.25rem; }
	.stats i { opacity: 0.3; font-style: normal; }
</style>
