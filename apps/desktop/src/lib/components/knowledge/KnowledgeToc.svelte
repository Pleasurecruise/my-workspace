<script lang="ts">
	import type { TocEntry } from "../../consumer";

	let { entries }: { entries: TocEntry[] } = $props();
	let activeId = $state("");

	function scrollTo(entry: TocEntry) {
		activeId = entry.id;
		const heading = document.getElementById(entry.id);
		if (heading !== null) heading.scrollIntoView({ behavior: "smooth", block: "start" });
	}
</script>

{#if entries.length > 0}
	<details>
		<summary>On this page</summary>
	<nav aria-label="Table of contents">
		{#each entries as entry (entry.id)}
			<button class:active={activeId === entry.id} style:padding-left={`${Math.max(0, entry.depth - 2) * 10}px`} onclick={() => scrollTo(entry)} title={entry.text}>
				<span></span><b>{entry.text}</b>
			</button>
		{/each}
	</nav>
	</details>
{/if}

<style>
	details { margin-block: 1.5rem; padding: 0.75rem 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
	summary { color: var(--color-muted-foreground); font-size: 0.875rem; cursor: pointer; }
	nav { display: grid; gap: 0.25rem; margin-top: 0.75rem; }
	button { display: flex; width: 100%; align-items: center; gap: 0.5rem; padding-block: 0.25rem; border: 0; background: transparent; color: var(--color-muted-foreground); cursor: pointer; text-align: left; }
	button span { width: 1.75rem; height: 0.2rem; flex: none; border-radius: var(--radius-full); background: currentColor; opacity: 0.2; }
	button b { overflow: hidden; font-size: 0.68rem; font-weight: 400; text-overflow: ellipsis; white-space: nowrap; opacity: 1; }
	button.active { color: var(--color-foreground); }
	button.active span { opacity: 0.9; }
</style>
