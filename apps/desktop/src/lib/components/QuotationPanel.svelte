<script lang="ts">
	import { Quote } from "@lucide/svelte";
	import type { Quotation } from "../consumer";
	let { quotation, error }: { quotation: Quotation | null; error: string | null } = $props();
</script>

<article class="quotation-panel" aria-label="Random quotation">
	<header><Quote size={15} /><h2>Random Quotation</h2></header>
	{#if quotation !== null}
		<blockquote>{quotation.content}</blockquote>
		<footer><span>— {quotation.author}</span>{#if quotation.tags.length > 0}<small>{quotation.tags.join(" · ")}</small>{/if}</footer>
	{:else}<p>{error === null ? "Loading…" : error}</p>{/if}
</article>

<style>
	.quotation-panel { display: grid; min-width: 0; min-height: 8.5rem; align-content: center; box-sizing: border-box; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.75rem; color: var(--color-muted-foreground); }
	h2 { margin: 0; font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	blockquote { margin: 0; font-family: var(--font-serif); font-size: 0.9rem; line-height: 1.5; }
	footer { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; margin-top: 0.7rem; color: var(--color-muted-foreground); font-size: 0.6rem; }
	footer small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	p { margin: 0; color: var(--color-muted-foreground); font-size: 0.65rem; text-align: center; }
</style>
