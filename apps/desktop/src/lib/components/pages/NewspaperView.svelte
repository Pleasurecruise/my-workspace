<script lang="ts">
	import { openUrl } from "@tauri-apps/plugin-opener";
	import PageSkeleton from "../layout/PageSkeleton.svelte";
	import type { NewspaperDaily } from "@/lib/contracts/newspaper";

	let { daily, error, loading, onretry }: { daily: NewspaperDaily | null; error: string | null; loading: boolean; onretry: () => void } = $props();

	const dayFormatter = new Intl.DateTimeFormat("en-US", { weekday: "long", year: "numeric", month: "long", day: "numeric" });
	const timeFormatter = new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit" });

	function openLink(event: MouseEvent & { currentTarget: HTMLAnchorElement }) {
		event.preventDefault();
		void openUrl(event.currentTarget.href);
	}
</script>

{#if daily === null && error === null}
	<PageSkeleton view="newspaper" title="Newspaper" description="AI 日报" />
{:else}
<div class="newspaper-view">
	<header class="page-header"><div><h1 class="page-title">Newspaper</h1><p class="page-description">AI 日报</p></div></header>
	{#if daily === null}
		<section class="empty" aria-label="AI 日报">
			<p>AIHOT Daily</p>
			<h2>Could not load the daily</h2>
			<span role="alert">{error}</span>
			<button type="button" disabled={loading} onclick={onretry}>Retry</button>
		</section>
	{:else}
		<section class="paper" lang="zh-CN" aria-label="AI 日报">
			<div class="edition-line">
				<strong>AIHOT Daily</strong>
				<span>AI 日报</span>
				<time datetime={daily.date}>{dayFormatter.format(new Date(`${daily.date}T00:00:00`))}</time>
			</div>
			<div class="rule"><span></span><i></i><span></span></div>
			{#if daily.lead !== null}
				<header class="cover">
					<p>Lead story · No. {daily.date.replaceAll("-", "")}</p>
					<h2>{daily.lead.title}</h2>
					<p class="deck">{daily.lead.paragraph}</p>
				</header>
			{/if}
			{#if error !== null}<p class="notice" role="alert">{error}</p>{/if}
			{#each daily.sections as section, index (index)}
				<section class="section" aria-label={section.label}>
					<h3>{section.label}</h3>
					<ol>
						{#each section.items as item, itemIndex (itemIndex)}
							<li>
								<a href={item.url} onclick={openLink} onauxclick={openLink}>{item.title}</a>
								<p>{item.summary}</p>
								<small>{item.source}</small>
							</li>
						{/each}
					</ol>
				</section>
			{/each}
			{#if daily.flashes.length > 0}
				<section class="section flashes" aria-label="快讯">
					<h3>快讯</h3>
					<ul>
						{#each daily.flashes as flash, index (index)}
							<li>
								<time datetime={flash.publishedAt}>{timeFormatter.format(new Date(flash.publishedAt))}</time>
								<a href={flash.url} onclick={openLink} onauxclick={openLink}>{flash.title}</a>
								<small>{flash.source}</small>
							</li>
						{/each}
					</ul>
				</section>
			{/if}
			<footer>
				<span>Generated {timeFormatter.format(new Date(daily.generatedAt))}</span>
				<a href={daily.url} onclick={openLink} onauxclick={openLink}>Source · AIHOT</a>
			</footer>
		</section>
	{/if}
</div>
{/if}

<style>
	.newspaper-view { min-height: calc(100vh - 7rem); }
	.paper { width: min(100%, 58rem); box-sizing: border-box; margin: 0 auto; padding: clamp(1.5rem, 4vw, 3.5rem); background: color-mix(in srgb, var(--color-muted) 42%, var(--color-background)); color: var(--color-foreground); }
	.paper a { color: inherit; text-decoration: none; }
	.paper a:hover { color: var(--color-accent); }
	.edition-line { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 1rem; color: var(--color-muted-foreground); font-family: var(--font-sans); font-size: 0.68rem; letter-spacing: 0.08em; text-transform: uppercase; }
	.edition-line strong { color: var(--color-accent); font-family: var(--font-serif); font-size: 0.82rem; font-weight: 500; letter-spacing: 0.12em; }
	.edition-line time { justify-self: end; }
	.rule { display: grid; grid-template-columns: 1fr 0.35rem 1fr; align-items: center; gap: 0.5rem; margin: 0.75rem 0 3rem; }
	.rule span { height: 1px; background: var(--color-border-strong); }
	.rule i { width: 0.35rem; height: 0.35rem; rotate: 45deg; background: var(--color-accent); }
	.cover { max-width: 48rem; margin: 0 auto 3rem; padding-bottom: 2rem; border-bottom: 1px solid var(--color-border); }
	.cover > p:first-child { margin: 0 0 1rem; color: var(--color-accent); font-family: var(--font-sans); font-size: 0.7rem; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase; }
	.cover h2 { margin: 0; font-family: var(--font-serif); font-size: clamp(2rem, 5vw, 3.75rem); font-weight: 500; letter-spacing: -0.035em; line-height: 1.12; }
	.deck { max-width: 42rem; margin: 1.5rem 0 0; color: var(--color-muted-foreground); font-family: var(--font-serif); font-size: 1.05rem; line-height: 1.55; }
	.notice { max-width: 48rem; margin: 0 auto 2rem; color: var(--color-error); font-size: 0.8rem; }
	.section { max-width: 48rem; margin: 0 auto 2.5rem; font-family: var(--font-sans); }
	.section h3 { margin: 0 0 1rem; padding-left: 0.75rem; border-left: 3px solid var(--color-accent); font-family: var(--font-serif); font-size: 1.3rem; font-weight: 500; line-height: 1.28; }
	.section ol, .section ul { display: grid; gap: 1.25rem; margin: 0; padding: 0; list-style: none; }
	.section li a { font-family: var(--font-serif); font-size: 1.05rem; font-weight: 500; line-height: 1.4; }
	.section li p { margin: 0.4rem 0; color: var(--color-muted-foreground); font-size: 0.9rem; line-height: 1.62; letter-spacing: 0.018em; }
	.section small { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.65rem; letter-spacing: 0.04em; }
	.flashes ul { gap: 0.75rem; }
	.flashes li { display: grid; grid-template-columns: 3rem 1fr; column-gap: 0.75rem; align-items: baseline; }
	.flashes li a { font-family: var(--font-sans); font-size: 0.92rem; }
	.flashes time { color: var(--color-accent); font-family: var(--font-mono); font-size: 0.72rem; }
	.flashes small { grid-column: 2; }
	.paper footer { display: flex; justify-content: space-between; gap: 1rem; max-width: 48rem; margin: 3rem auto 0; padding-top: 1rem; border-top: 1px solid var(--color-border-strong); color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.62rem; letter-spacing: 0.05em; text-transform: uppercase; }
	.empty { display: grid; width: min(100%, 48rem); min-height: 18rem; align-content: center; justify-items: center; gap: 0.8rem; box-sizing: border-box; margin: 0 auto; padding: 2rem 0; border-block: 1px solid var(--color-border); text-align: center; }
	.empty p { margin: 0; color: var(--color-accent); font-size: 0.7rem; letter-spacing: 0.12em; text-transform: uppercase; }
	.empty h2 { margin: 0; font-family: var(--font-serif); font-size: 2rem; font-weight: 500; }
	.empty span { color: var(--color-muted-foreground); font-size: 0.8rem; }
	.empty button { padding: 0.35rem 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: transparent; color: var(--color-foreground); cursor: pointer; font-size: 0.75rem; }
	.empty button:hover { background: var(--color-muted); }
	.empty button:disabled { cursor: wait; opacity: 0.6; }
	@media (max-width: 640px) {
		.edition-line { grid-template-columns: 1fr auto; }
		.edition-line span { display: none; }
		.cover h2 { font-size: 2rem; }
		.paper footer { align-items: flex-start; flex-direction: column; }
	}
</style>
