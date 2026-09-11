<script lang="ts">
	import { ListTree } from "@lucide/svelte";
	import type { TocEntry } from "../../consumer";

	let { entries }: { entries: TocEntry[] } = $props();
	let panel = $state<HTMLDetailsElement | null>(null);

	function scrollTo(entry: TocEntry) {
		if (panel !== null) panel.open = false;
		const heading = document.getElementById(entry.id);
		if (heading !== null) heading.scrollIntoView({ behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth", block: "start" });
	}
</script>

<svelte:window onkeydown={(event) => { if (panel !== null && panel.open && event.key === "Escape") { panel.open = false; panel.querySelector("summary")?.focus(); } }} />

{#if entries.length > 0}
	<details bind:this={panel} onfocusout={(event) => { if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) event.currentTarget.open = false; }}>
		<summary aria-label="Table of contents" title="Table of contents"><ListTree size={16} /></summary>
		<nav aria-label="Table of contents">
			{#each entries as entry (entry.id)}
				<button type="button" style:padding-left={`${0.75 + Math.max(0, entry.depth - 2) * 0.625}rem`} onclick={() => scrollTo(entry)}>{entry.text}</button>
			{/each}
		</nav>
	</details>
{/if}

<style>
	details { position: relative; }
	summary { display: flex; align-items: center; justify-content: center; padding: 0.25rem; list-style: none; color: var(--color-muted-foreground); cursor: pointer; }
	summary::-webkit-details-marker { display: none; }
	summary:hover, details[open] summary { color: var(--color-foreground); }
	summary:focus-visible, button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	nav { position: absolute; z-index: 10; top: calc(100% + 0.5rem); right: 0; display: grid; width: min(18rem, 65vw); max-height: min(24rem, 60vh); overflow-y: auto; overscroll-behavior: contain; padding: 0.35rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); box-shadow: var(--shadow-lg); }
	button { padding: 0.5rem 0.75rem; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); font: 0.75rem var(--font-sans); cursor: pointer; text-align: left; overflow-wrap: anywhere; }
	button:hover { background: var(--color-muted); color: var(--color-foreground); }
</style>
