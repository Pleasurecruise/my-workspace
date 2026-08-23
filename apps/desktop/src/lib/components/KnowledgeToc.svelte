<script lang="ts">
	import type { TocEntry } from "../consumer";

	let { entries }: { entries: TocEntry[] } = $props();
	let activeId = $state("");

	function scrollTo(entry: TocEntry) {
		activeId = entry.id;
		const heading = document.getElementById(entry.id);
		if (heading !== null) heading.scrollIntoView({ behavior: "smooth", block: "start" });
	}
</script>

{#if entries.length > 0}
	<nav aria-label="Table of contents">
		{#each entries as entry (entry.id)}
			<button class:active={activeId === entry.id} style:padding-left={`${Math.max(0, entry.depth - 2) * 10}px`} onclick={() => scrollTo(entry)} title={entry.text}>
				<span></span><b>{entry.text}</b>
			</button>
		{/each}
	</nav>
{/if}

<style>
	nav { position: fixed; top: 50%; width: 10rem; max-height: calc(100vh - 16rem); margin-left: -12rem; translate: 0 -50%; overflow-y: auto; }
	button { display: flex; width: 100%; align-items: center; gap: 0.5rem; padding-block: 0.25rem; border: 0; background: transparent; color: var(--color-muted-foreground); cursor: pointer; text-align: left; }
	button span { width: 1.75rem; height: 0.2rem; flex: none; border-radius: var(--radius-full); background: currentColor; opacity: 0.2; }
	button b { overflow: hidden; font-size: 0.68rem; font-weight: 400; text-overflow: ellipsis; white-space: nowrap; opacity: 0; transition: opacity var(--duration-fast); }
	nav:hover button b { opacity: 1; }
	button.active { color: var(--color-foreground); }
	button.active span { opacity: 0.9; }
	@media (max-width: 1180px) { nav { display: none; } }
</style>
