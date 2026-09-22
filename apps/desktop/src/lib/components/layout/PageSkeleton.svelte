<script lang="ts">
	let { view, title = "", description = "Loading…" }: { view: "memos" | "moment" | "knowledge" | "newspaper" | "music"; title?: string; description?: string } = $props();
</script>

<section class="page-skeleton" aria-label={`Loading ${view}`} aria-busy="true" role="status">
	{#if title}
		<header class="page-header"><div><h1>{title}</h1><p class="page-description">{description}</p></div></header>
	{/if}
	<div class="placeholders" class:newspaper={view === "newspaper"} class:photos={view === "moment"} aria-hidden="true">
		{#if view === "moment"}
			{#each [1, 2, 3, 4, 5, 6] as item (item)}<div class="photo pulse" class:tall={item % 2 === 0}></div>{/each}
		{:else if view === "memos"}
			<div class="composer pulse"></div>
			{#each [1, 2, 3] as item (item)}<div class="memo"><div class="line short pulse"></div><div class="line pulse"></div><div class="line pulse"></div><div class="line short pulse"></div></div>{/each}
		{:else if view === "knowledge"}
			<div class="line short pulse"></div>
			{#each [1, 2, 3, 4, 5, 6] as item (item)}<div class="article-row"><div class="line date pulse"></div><div class="line pulse"></div></div>{/each}
		{:else if view === "music"}
			{#each [1, 2, 3, 4, 5, 6] as item (item)}<div class="track-row"><div class="cover pulse"></div><div class="line pulse"></div><div class="line date pulse"></div></div>{/each}
		{:else}
			<div class="masthead pulse"></div>
			{#each [1, 2, 3] as item (item)}<div class="paragraph"><div class="line short pulse"></div><div class="line pulse"></div><div class="line pulse"></div><div class="line pulse"></div></div>{/each}
		{/if}
	</div>
</section>

<style>
	.page-skeleton { width: 100%; min-width: 0; }
	.newspaper { width: min(100%, 58rem); box-sizing: border-box; margin-inline: auto; padding: clamp(1.5rem, 4vw, 3.5rem); }
	.placeholders, .memo, .paragraph { display: grid; gap: 0.85rem; }
	.placeholders { gap: 1.25rem; }
	.pulse { border-radius: var(--radius-md); background: var(--color-muted); animation: pulse var(--duration-skeleton) ease-in-out infinite alternate; }
	.line { height: 0.75rem; }
	.short { width: 40%; }
	.date { width: 4rem; }
	.composer { height: 8rem; }
	.memo { padding: 1.25rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); }
	.photos { grid-template-columns: repeat(3, minmax(0, 1fr)); align-items: start; gap: 0.65rem; }
	.photo { aspect-ratio: 4 / 3; }
	.photo.tall { aspect-ratio: 4 / 5; }
	.article-row { display: grid; grid-template-columns: 4rem minmax(0, 1fr); gap: 1rem; padding-block: 0.75rem; }
	.track-row { display: grid; grid-template-columns: 2.6rem minmax(0, 1fr) 4rem; gap: 1rem; align-items: center; }
	.cover { aspect-ratio: 1; }
	.masthead { height: 6rem; margin-bottom: 1.5rem; }
	.paragraph { margin-bottom: 1.5rem; }
	@media (max-width: 1023px) { .photos { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
	@media (max-width: 639px) { .photos { grid-template-columns: minmax(0, 1fr); } }
	@keyframes pulse { to { opacity: 0.45; } }
	@media (prefers-reduced-motion: reduce) { .pulse { animation: none; } }
</style>
