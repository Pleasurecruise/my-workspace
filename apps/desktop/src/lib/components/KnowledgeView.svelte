<script lang="ts">
	import { ArrowLeft, ArrowUp, Pencil, Plus } from "@lucide/svelte";
	import type { CommandResponse, CompiledKnowledge, KnowledgeDocument } from "../consumer";
	import KnowledgeHeader from "./KnowledgeHeader.svelte";
	import KnowledgeToc from "./KnowledgeToc.svelte";

	let { documents, loading, oncompile }: { documents: KnowledgeDocument[]; loading: boolean; oncompile: (source: string) => Promise<CommandResponse<CompiledKnowledge>> } = $props();
	let selected = $state<KnowledgeDocument | null>(null);
	let editing = $state(false);
	let draftTitle = $state("");
	let draftSummary = $state("");
	let draftSource = $state("");
	let compiling = $state(false);
	let error = $state("");

	type KnowledgeMonth = {
		month: number;
		entries: KnowledgeDocument[];
	};

	type KnowledgeYear = {
		year: number;
		count: number;
		months: KnowledgeMonth[];
	};

	let groups = $derived.by(() => {
		const years: KnowledgeYear[] = [];
		for (const document of documents) {
			const date = new Date(document.createdAt);
			const year = date.getFullYear();
			const month = date.getMonth();
			let yearGroup: KnowledgeYear | null = null;
			for (const candidate of years) {
				if (candidate.year === year) yearGroup = candidate;
			}
			if (yearGroup === null) {
				yearGroup = { year, count: 0, months: [] };
				years.push(yearGroup);
			}
			let monthGroup: KnowledgeMonth | null = null;
			for (const candidate of yearGroup.months) {
				if (candidate.month === month) monthGroup = candidate;
			}
			if (monthGroup === null) {
				monthGroup = { month, entries: [] };
				yearGroup.months.push(monthGroup);
			}
			monthGroup.entries.push(document);
			yearGroup.count += 1;
		}
		years.sort((left, right) => right.year - left.year);
		for (const year of years) {
			year.months.sort((left, right) => right.month - left.month);
		}
		return years;
	});

	function monthName(month: number) {
		return new Intl.DateTimeFormat("en-US", { month: "short" }).format(new Date(2020, month)).toUpperCase();
	}

	function startNew() {
		selected = null;
		draftTitle = "";
		draftSummary = "";
		draftSource = "";
		error = "";
		editing = true;
	}

	function startEdit(document: KnowledgeDocument) {
		draftTitle = document.title;
		draftSummary = document.summary;
		draftSource = document.source;
		error = "";
		editing = true;
	}

	function cancelEdit() {
		editing = false;
		error = "";
	}

	async function preview() {
		if (compiling || draftTitle.trim() === "") return;
		compiling = true;
		error = "";
		const response = await oncompile(draftSource);
		compiling = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		const current = selected;
		const now = new Date().toISOString();
		selected = {
			id: current === null ? "preview" : current.id,
			slug: current === null ? "preview" : current.slug,
			title: draftTitle.trim(),
			summary: draftSummary.trim(),
			tags: current === null ? [] : current.tags,
			visibility: current === null ? "private" : current.visibility,
			contentHash: current === null ? "" : current.contentHash,
			createdAt: current === null ? now : current.createdAt,
			updatedAt: now,
			source: draftSource,
			html: response.data.html,
			toc: response.data.toc,
		};
		editing = false;
	}
</script>

{#if editing}
	<section class="editor" aria-label="Knowledge editor">
		<header>
			<div><h1>{selected ? "Edit knowledge" : "New knowledge"}</h1><p>Markdown preview is compiled by Rust. R2 persistence will be connected separately.</p></div>
			<div class="editor-actions"><button onclick={cancelEdit}>Cancel</button><button class="preview" disabled={compiling || draftTitle.trim() === ""} onclick={preview}>{compiling ? "Compiling..." : "Preview"}</button></div>
		</header>
		{#if error}<p class="editor-error">{error}</p>{/if}
		<div class="fields">
			<label>Title<input bind:value={draftTitle} placeholder="Untitled knowledge" /></label>
			<label>Summary<input bind:value={draftSummary} placeholder="A short summary" /></label>
		</div>
		<label class="markdown">Markdown<textarea bind:value={draftSource} placeholder="Start writing..." spellcheck="true"></textarea></label>
	</section>
{:else if selected}
	<section class="reader" id="knowledge-article">
		<div class="article-heading">
			<KnowledgeHeader title={selected.title} text={selected.source} />
			<div class="mobile-actions">
				<button onclick={() => (selected = null)} aria-label="All knowledge"><ArrowLeft size={15} /></button>
				<button onclick={() => selected !== null && startEdit(selected)} aria-label="Edit"><Pencil size={15} /></button>
			</div>
		</div>
		<KnowledgeToc entries={selected.toc} />
		<article class="prose">{@html selected.html}</article>
		<aside class="article-actions" aria-label="Article actions">
			<button onclick={() => (selected = null)} title="All knowledge"><ArrowLeft size={16} /></button>
			<button onclick={() => selected !== null && startEdit(selected)} title="Edit"><Pencil size={16} /></button>
			<button onclick={() => {
				const article = document.getElementById("knowledge-article");
				if (article !== null) article.scrollIntoView({ behavior: "smooth" });
			}} title="Back to top"><ArrowUp size={16} /></button>
		</aside>
	</section>
{:else}
	<section class="index">
		<header class="index-header">
			<div><h1>knowledge</h1><span></span></div>
			<button onclick={startNew}><Plus size={12} /> new</button>
		</header>
		<p class="lede">Long-form writing — linked, searchable, and always in markdown.</p>

		{#each groups as group (group.year)}
			<section class="year">
				<h2>{group.year} <small>{group.count} entries</small></h2>
				{#each group.months as month (month.month)}
					<h3>{monthName(month.month)}</h3>
					<ol>
						{#each month.entries as entry (entry.id)}
							<li><time>{new Date(entry.createdAt).getDate().toString().padStart(2, "0")}</time><button onclick={() => (selected = entry)}>{entry.title}</button><span>{entry.tags.join(" · ")}</span></li>
						{/each}
					</ol>
				{/each}
			</section>
		{/each}
		{#if documents.length === 0 && !loading}<p class="empty">No knowledge documents found.</p>{/if}
	</section>
{/if}

<style>
	.index { width: min(100%, 45rem); margin: 0 auto; }
	.editor { width: min(100%, 52rem); margin: 0 auto; }
	.editor header { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
	.editor h1 { margin: 0; font-family: var(--font-serif); font-size: 1.5rem; }
	.editor header p { margin: 0.5rem 0 0; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.editor-actions { display: flex; gap: 0.5rem; }
	.editor-actions button { height: 2rem; padding: 0 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); font-size: 0.75rem; }
	.editor-actions .preview { border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.editor-actions button:disabled { opacity: 0.5; }
	.fields { display: grid; grid-template-columns: 3fr 2fr; gap: 1rem; margin-bottom: 1rem; }
	.editor label { display: grid; gap: 0.5rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.65rem; letter-spacing: 0.08em; text-transform: uppercase; }
	.editor input, .editor textarea { box-sizing: border-box; width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.875rem; letter-spacing: normal; text-transform: none; }
	.editor input { height: 2.5rem; padding: 0 0.75rem; }
	.editor textarea { min-height: calc(100vh - 18rem); padding: 1rem; resize: vertical; font-family: var(--font-mono); line-height: 1.65; }
	.editor input:focus, .editor textarea:focus { border-color: var(--color-border-strong); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 16%, transparent); }
	.editor-error { padding: 0.75rem; border: 1px solid var(--color-error); border-radius: var(--radius-md); color: var(--color-error); font-size: 0.75rem; }
	.index-header { display: flex; align-items: flex-start; justify-content: space-between; }
	.index-header div { position: relative; }
	.index-header h1 { margin: 0; font-family: var(--font-serif); font-size: 1.75rem; line-height: 1; }
	.index-header div span { position: absolute; bottom: -0.4rem; left: 0; width: 2rem; height: 2px; border-radius: var(--radius-full); background: var(--color-accent); }
	.index-header button, .mobile-actions button, .article-actions button { display: inline-flex; align-items: center; gap: 0.25rem; border: 0; background: transparent; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.lede { margin: 1.25rem 0 2rem; color: var(--color-muted-foreground); font-size: 0.875rem; }
	.year { margin: 1.25rem 0; }
	.year h2 { margin: 0 0 1rem; font-size: 0.875rem; font-weight: 500; }
	.year h2 small { margin-left: 0.25rem; color: var(--color-muted-foreground); font-weight: 400; }
	.year h3 { margin: 0 0 0.5rem; color: var(--color-muted-foreground); font-size: 0.7rem; letter-spacing: 0.06em; }
	ol { display: grid; gap: 0.375rem; margin: 0 0 1rem; padding: 0; list-style: none; }
	li { display: grid; grid-template-columns: 2.25rem minmax(0, 1fr) max-content; gap: 0.75rem; }
	li time { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.875rem; }
	li button { overflow: hidden; border: 0; background: transparent; color: var(--color-foreground); cursor: pointer; font-size: 0.875rem; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
	li button:hover { color: var(--color-accent); text-decoration: underline; text-underline-offset: 0.25rem; }
	li span { color: var(--color-muted-foreground); font-size: 0.75rem; }
	.empty { padding: 4rem 0; color: var(--color-muted-foreground); font-size: 0.875rem; text-align: center; }
	.reader { position: relative; width: min(100%, 40.625rem); margin: 1rem auto 0; }
	.article-heading { display: flex; align-items: center; gap: 1rem; }
	.mobile-actions { display: none; margin-left: auto; }
	.prose { min-width: 0; margin-top: 2rem; color: var(--color-foreground); font-size: 0.95rem; line-height: 1.8; overflow-wrap: anywhere; }
	.prose :global(img) { max-width: 100%; height: auto; }
	.prose :global(pre) { overflow-x: auto; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-muted); }
	.prose :global(a) { color: var(--color-accent); }
	.prose :global(h1), .prose :global(h2), .prose :global(h3) { scroll-margin-top: 4rem; line-height: 1.35; }
	.article-actions { position: fixed; top: 50%; display: grid; gap: 0.25rem; width: 2.5rem; margin-left: calc(40.625rem + 2rem); translate: 0 -50%; }
	.article-actions button { display: grid; width: 1.75rem; height: 1.75rem; place-items: center; border-radius: var(--radius-full); cursor: pointer; }
	.article-actions button:hover { color: var(--color-foreground); }
	@media (max-width: 1180px) { .article-actions { display: none; } .mobile-actions { display: flex; } }
	@media (max-width: 640px) { .editor header { display: grid; } .fields { grid-template-columns: 1fr; } }
</style>
