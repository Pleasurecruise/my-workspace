<script lang="ts">
	import { tick } from "svelte";
	import type { KnowledgeDocument, NewspaperIssues } from "../consumer";

	let { documents, issues, loading }: { documents: KnowledgeDocument[]; issues: NewspaperIssues; loading: boolean } = $props();

	type EditionKind = "developer" | "personal";
	const editionLabels: Record<EditionKind, string> = {
		developer: "程序员日报",
		personal: "每日日报",
	};

	let viewElement = $state<HTMLDivElement | null>(null);
	let selectedEdition = $state<EditionKind>("developer");
	let turnDirection = $state<"left" | "right">("right");
	const issue = $derived(documents.find((document) => document.id === issues[selectedEdition]) ?? null);

	async function turnPage(edition: EditionKind) {
		turnDirection = edition === "developer" ? "left" : "right";
		selectedEdition = edition;
		await tick();
		viewElement?.closest("main")?.scrollTo({ top: 0, behavior: "instant" });
	}
</script>

<div bind:this={viewElement} class="newspaper-view">
	<button
		class="page-arrow previous"
		type="button"
		disabled={selectedEdition === "developer"}
		aria-label="翻到程序员日报"
		title="程序员日报"
		onclick={() => void turnPage("developer")}
	><span aria-hidden="true"></span></button>
	<button
		class="page-arrow next"
		type="button"
		disabled={selectedEdition === "personal"}
		aria-label="翻到每日日报"
		title="每日日报"
		onclick={() => void turnPage("personal")}
	><span aria-hidden="true"></span></button>

	<div class="page-stage">
		{#key selectedEdition}
			{#if issue === null}
				<section class="empty edition-page" class:turn-right={turnDirection === "right"} class:turn-left={turnDirection === "left"} aria-label={editionLabels[selectedEdition]}>
					<p>{editionLabels[selectedEdition]}</p>
					<h1>{editionLabels[selectedEdition]}尚未发布</h1>
					<span>{loading ? "正在检查 my-knowledge…" : "发布后，最新一期会出现在这里。"}</span>
				</section>
			{:else}
				<section class="paper edition-page" class:turn-right={turnDirection === "right"} class:turn-left={turnDirection === "left"} lang="zh-CN" aria-label={editionLabels[selectedEdition]}>
					<div class="edition-line">
						<strong>{selectedEdition === "developer" ? "Vesper Developer Daily" : "Vesper Personal Daily"}</strong>
						<span>{editionLabels[selectedEdition]}</span>
						<time datetime={issue.createdAt}>{new Intl.DateTimeFormat("en-US", { weekday: "long", year: "numeric", month: "long", day: "numeric" }).format(new Date(issue.createdAt))}</time>
					</div>
					<div class="rule"><span></span><i></i><span></span></div>
					<header class="cover">
						<p>Daily intelligence · No. {new Intl.DateTimeFormat("en-CA", { year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date(issue.createdAt)).replaceAll("/", "")}</p>
						<h1>{issue.title}</h1>
						<p class="deck">{issue.summary}</p>
					</header>
					<article class="copy">{@html issue.html}</article>
					<footer>
						<span>Updated {new Intl.DateTimeFormat("en-US", { hour: "2-digit", minute: "2-digit" }).format(new Date(issue.updatedAt))}</span>
						<span>Vesper · my-knowledge</span>
					</footer>
				</section>
			{/if}
		{/key}
	</div>
</div>

<style>
	.newspaper-view { min-height: calc(100vh - 7rem); }
	.page-stage { perspective: 1400px; }
	.page-arrow { position: fixed; top: 50%; z-index: 5; display: grid; width: 2.25rem; height: 2.25rem; place-items: center; padding: 0; translate: 0 -50%; border: 0; border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); cursor: pointer; opacity: 0.55; }
	.page-arrow.previous { left: calc(var(--sidebar-width, 15rem) + max(1rem, (100vw - var(--sidebar-width, 15rem) - 66rem) / 2) + 1.65rem); }
	.page-arrow.next { right: calc(max(1rem, (100vw - var(--sidebar-width, 15rem) - 66rem) / 2) + 1.65rem); }
	.page-arrow span { width: 0.5rem; height: 0.5rem; border-top: 1.5px solid currentColor; border-right: 1.5px solid currentColor; }
	.page-arrow.previous span { rotate: -135deg; }
	.page-arrow.next span { rotate: 45deg; }
	.page-arrow:hover:not(:disabled) { background: color-mix(in srgb, var(--color-muted) 72%, transparent); color: var(--color-accent); opacity: 1; }
	.page-arrow.previous:hover:not(:disabled) span { translate: -0.1rem 0; }
	.page-arrow.next:hover:not(:disabled) span { translate: 0.1rem 0; }
	.page-arrow:disabled { visibility: hidden; }
	.edition-page { min-height: 0; backface-visibility: hidden; }
	.edition-page.turn-right { transform-origin: left top; animation: page-in-right 460ms ease-out both; }
	.edition-page.turn-left { transform-origin: right top; animation: page-in-left 460ms ease-out both; }
	.paper { width: min(100%, 58rem); box-sizing: border-box; margin: 0 auto; padding: clamp(1.5rem, 4vw, 3.5rem); background: color-mix(in srgb, var(--color-muted) 42%, var(--color-background)); color: var(--color-foreground); }
	.edition-line { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 1rem; color: var(--color-muted-foreground); font-family: var(--font-sans); font-size: 0.68rem; letter-spacing: 0.08em; text-transform: uppercase; }
	.edition-line strong { color: var(--color-accent); font-family: var(--font-serif); font-size: 0.82rem; font-weight: 500; letter-spacing: 0.12em; }
	.edition-line time { justify-self: end; }
	.rule { display: grid; grid-template-columns: 1fr 0.35rem 1fr; align-items: center; gap: 0.5rem; margin: 0.75rem 0 3rem; }
	.rule span { height: 1px; background: var(--color-border-strong); }
	.rule i { width: 0.35rem; height: 0.35rem; rotate: 45deg; background: var(--color-accent); }
	.cover { max-width: 48rem; margin: 0 auto 3rem; padding-bottom: 2rem; border-bottom: 1px solid var(--color-border); }
	.cover > p:first-child { margin: 0 0 1rem; color: var(--color-accent); font-family: var(--font-sans); font-size: 0.7rem; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase; }
	.cover h1 { margin: 0; font-family: var(--font-serif); font-size: clamp(2.25rem, 6vw, 4.5rem); font-weight: 500; letter-spacing: -0.035em; line-height: 1.08; }
	.deck { max-width: 42rem; margin: 1.5rem 0 0; color: var(--color-muted-foreground); font-family: var(--font-serif); font-size: 1.05rem; line-height: 1.55; }
	.copy { max-width: 48rem; margin: 0 auto; font-family: var(--font-sans); font-size: 0.95rem; line-height: 1.62; letter-spacing: 0.018em; overflow-wrap: break-word; }
	.copy :global(h1), .copy :global(h2), .copy :global(h3), .copy :global(h4), .copy :global(h5), .copy :global(h6) { margin: 2.25em 0 0.65em; font-family: var(--font-serif); font-weight: 500; line-height: 1.28; }
	.copy :global(h1) { padding-left: 0.75rem; border-left: 3px solid var(--color-accent); font-size: 1.7em; }
	.copy :global(h2) { padding-left: 0.75rem; border-left: 3px solid var(--color-accent); font-size: 1.4em; }
	.copy :global(h3) { color: var(--color-foreground); font-size: 1.17em; }
	.copy :global(h4) { color: var(--color-muted-foreground); font-size: 1.05em; }
	.copy :global(p) { margin: 0 0 1em; }
	.copy :global(ul), .copy :global(ol) { margin: 0.75em 0 1em; padding-left: 1.4rem; }
	.copy :global(li) { margin: 0.35em 0; }
	.copy :global(li::marker) { color: var(--color-accent); }
	.copy :global(strong) { font-weight: 600; }
	.copy :global(blockquote) { margin: 1.5rem 0; padding: 0.4rem 0 0.4rem 1rem; border-left: 2px solid var(--color-accent); color: var(--color-muted-foreground); }
	.copy :global(a) { color: var(--color-accent); text-decoration-thickness: 1px; text-underline-offset: 0.2em; }
	.copy :global(hr) { margin: 2.5rem 0; border: 0; border-top: 1px solid var(--color-border); }
	.copy :global(pre) { overflow-x: auto; margin: 1.5rem 0; padding: 1rem; border-radius: var(--radius-sm); background: var(--color-background); font-family: var(--font-mono); font-size: 0.8rem; line-height: 1.6; }
	.copy :global(:not(pre) > code) { padding: 0.15em 0.35em; border-radius: var(--radius-sm); background: var(--color-background); font-family: var(--font-mono); font-size: 0.88em; }
	.copy :global(table) { display: block; overflow-x: auto; width: 100%; margin: 1.5rem 0; border-collapse: collapse; font-size: 0.88em; }
	.copy :global(th), .copy :global(td) { padding: 0.6rem; border-bottom: 1px solid var(--color-border); text-align: left; }
	.paper footer { display: flex; justify-content: space-between; gap: 1rem; max-width: 48rem; margin: 3rem auto 0; padding-top: 1rem; border-top: 1px solid var(--color-border-strong); color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.62rem; letter-spacing: 0.05em; text-transform: uppercase; }
	.empty { display: grid; width: min(100%, 48rem); min-height: calc(100vh - 7rem); align-content: center; box-sizing: border-box; margin: 0 auto; padding: 2rem 0; border-block: 1px solid var(--color-border); text-align: center; }
	.empty p { margin: 0; color: var(--color-accent); font-size: 0.7rem; letter-spacing: 0.12em; text-transform: uppercase; }
	.empty h1 { margin: 0.8rem 0; font-family: var(--font-serif); font-size: 2rem; font-weight: 500; }
	.empty span { color: var(--color-muted-foreground); font-size: 0.8rem; }
	@keyframes page-in-right { from { opacity: 0.32; transform: rotateY(-14deg) translateX(1rem); } to { opacity: 1; transform: rotateY(0) translateX(0); } }
	@keyframes page-in-left { from { opacity: 0.32; transform: rotateY(14deg) translateX(-1rem); } to { opacity: 1; transform: rotateY(0) translateX(0); } }
	@media (prefers-reduced-motion: reduce) {
		.edition-page { animation: none !important; }
	}
	@media (max-width: 640px) {
		.edition-line { grid-template-columns: 1fr auto; }
		.edition-line span { display: none; }
		.cover h1 { font-size: 2.25rem; }
		.paper footer { align-items: flex-start; flex-direction: column; }
		.page-arrow { width: 2rem; height: 2rem; background: color-mix(in srgb, var(--color-background) 82%, transparent); opacity: 0.72; backdrop-filter: blur(8px); }
	}
	@media (max-width: 767px) {
		.page-arrow.previous { left: 2.1rem; }
		.page-arrow.next { right: 2.1rem; }
	}
</style>
