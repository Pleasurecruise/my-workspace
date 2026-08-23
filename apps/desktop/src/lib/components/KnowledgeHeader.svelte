<script lang="ts">
	import { Clock, Type } from "@lucide/svelte";

	let { title, text }: { title: string; text: string } = $props();

	let stats = $derived.by(() => {
		const cjk = Array.from(text.matchAll(/[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/gu)).length;
		const latinMatches = text
			.replaceAll(/[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/gu, " ")
			.match(/[A-Za-z0-9]+(?:['-][A-Za-z0-9]+)*/g);
		const latin = latinMatches === null ? 0 : latinMatches.length;
		return { count: cjk + latin, minutes: Math.max(1, Math.ceil(cjk / 350 + latin / 200)) };
	});
</script>

<header>
	<h1>{title}</h1>
	{#if stats.count > 0}
		<div class="stats">
			<span><Type size={13} />{stats.count}</span><i>·</i><span><Clock size={13} />{stats.minutes} min</span>
		</div>
	{/if}
</header>

<style>
	header { min-width: 0; }
	h1 { margin: 0; color: var(--color-foreground); font-size: clamp(1.25rem, 3vw, 1.5rem); font-weight: 500; line-height: 1.3; }
	.stats { display: flex; align-items: center; gap: 0.4rem; margin-top: 0.5rem; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.stats span { display: inline-flex; align-items: center; gap: 0.25rem; }
	.stats i { opacity: 0.3; font-style: normal; }
</style>
