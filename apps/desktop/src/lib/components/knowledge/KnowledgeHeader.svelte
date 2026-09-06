<script lang="ts">
	import type { Snippet } from "svelte";
	import { Clock, Type } from "@lucide/svelte";

	let { title, text, actions }: { title: string; text: string; actions: Snippet } = $props();

	let stats = $derived.by(() => {
		const cjk = Array.from(text.matchAll(/[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/gu)).length;
		const latinMatches = text
			.replaceAll(/[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/gu, " ")
			.match(/[A-Za-z0-9]+(?:['-][A-Za-z0-9]+)*/g);
		const latin = latinMatches === null ? 0 : latinMatches.length;
		return { count: cjk + latin, minutes: Math.max(1, Math.ceil(cjk / 350 + latin / 200)) };
	});
</script>

<header class="page-header"><div>
	<h1>{title}</h1>
	{#if stats.count > 0}
		<div class="stats">
			<span><Type size={13} />{stats.count}</span><i>·</i><span><Clock size={13} />{stats.minutes} min</span>
		</div>
	{/if}
</div>
	{@render actions()}
</header>

<style>
	header { min-width: 0; }
	h1 { margin: 0; }
	.stats { display: flex; align-items: center; gap: 0.4rem; margin-top: 0.5rem; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.stats span { display: inline-flex; align-items: center; gap: 0.25rem; }
	.stats i { opacity: 0.3; font-style: normal; }
</style>
