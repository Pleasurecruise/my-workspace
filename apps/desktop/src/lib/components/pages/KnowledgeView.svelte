<script module lang="ts">
	import type { KnowledgeDocument } from "../../consumer";

	let selected = $state<KnowledgeDocument | null>(null);
	let destination = $state<{ id: string; fragment: string } | null>(null);
	let articleTrail = $state<KnowledgeDocument[]>([]);
	let editing = $state(false);
	let draftTitle = $state("");
	let draftSummary = $state("");
	let draftTags = $state("");
	let draftSource = $state("");
	let draftVisibility = $state<KnowledgeDocument["visibility"]>("public");
	let saving = $state(false);
	let error = $state("");

	export function selectKnowledgeArticle(document: KnowledgeDocument, fragment = ""): string | null {
		if (editing || saving) return "Finish or cancel the current article draft before opening another article.";
		destination = { id: document.id, fragment };
		if (document.id === selected?.id) return null;
		const index = articleTrail.findIndex((article) => article.id === document.id);
		articleTrail = index >= 0 ? articleTrail.slice(0, index) : selected !== null ? [...articleTrail, selected].slice(-8) : [];
		selected = document;
		return null;
	}

	function returnToArticle() {
		destination = null;
		selected = articleTrail.at(-1) ?? null;
		articleTrail = articleTrail.slice(0, -1);
	}
</script>

<script lang="ts">
	import { onDestroy } from "svelte";
	import { openArticleLinks, preloadArticles } from "../knowledge/links";
	import { mediaPlayers } from "../knowledge/media";
	import { ArrowLeft, Check, Link, Pencil, Plus } from "@lucide/svelte";
	import type { CommandResponse, KnowledgeDraft, KnowledgeUpdate, KnowledgeEntry } from "../../consumer";
	import KnowledgeHeader from "../knowledge/KnowledgeHeader.svelte";
	import KnowledgeToc from "../knowledge/KnowledgeToc.svelte";
	import RichMarkdownEditor from "../knowledge/RichMarkdownEditor.svelte";

	let linkError = $state<string | null>(null);
	let articleElement = $state<HTMLElement | null>(null);
	$effect(() => {
		if (!destination || !articleElement || articleElement.dataset.articleId !== destination.id) return;
		let fragment = destination.fragment;
		try { fragment = decodeURIComponent(fragment); } catch { /* Malformed fragments can still match a literal heading ID. */ }
		const heading = fragment ? Array.from(articleElement.querySelectorAll<HTMLElement>("[id]")).find((element) => element.id === fragment) : undefined;
		if (heading) heading.scrollIntoView({ behavior: "instant", block: "start" });
		else articleElement.closest("main")?.scrollTo({ top: 0, behavior: "instant" });
		destination = null;
	});
	let copiedArticle = $state<string | null>(null);
	let copyError = $state<string | null>(null);
	let copyRequest = 0;
	let copyTarget: string | null = null;
	$effect(() => { const id = selected?.id ?? null; if (id === copyTarget) return; copyTarget = id; copiedArticle = null; copyError = null; copyRequest++; });
	async function copyArticleLink() {
		if (selected === null) return;
		const article = selected;
		const request = ++copyRequest;
		copiedArticle = null;
		copyError = null;
		await navigator.clipboard.writeText(`https://knowledge.you-find.me/articles/${encodeURIComponent(article.id)}`).then(() => {
			if (request === copyRequest) copiedArticle = article.id;
		}, () => {
			if (request === copyRequest) copyError = "Could not copy the article link. Please try again.";
		});
	}

	let {
		documents,
		loading,
		onread,
		oncreate,
		onupdate,
	}: {
		documents: KnowledgeEntry[];
		loading: boolean;
		onread: (id: string, expectedHash: string | null) => Promise<CommandResponse<KnowledgeDocument>>;
		oncreate: (input: KnowledgeDraft) => Promise<CommandResponse<KnowledgeDocument>>;
		onupdate: (id: string, input: KnowledgeUpdate) => Promise<CommandResponse<KnowledgeDocument>>;
	} = $props();

	type KnowledgeMonth = {
		month: number;
		entries: KnowledgeEntry[];
	};

	type KnowledgeYear = {
		year: number;
		count: number;
		months: KnowledgeMonth[];
	};
	let groups = $derived.by(() => {
		const years: KnowledgeYear[] = [];
		for (const document of documents) {
			if (document.newspaperEdition !== null) continue;
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

	let openingId = $state<string | null>(null);
	let readRequest = 0;
	onDestroy(() => { readRequest++; copyRequest++; });
	async function openEntry(entry: KnowledgeEntry) {
		const request = ++readRequest;
		openingId = entry.id;
		error = "";
		const response = await onread(entry.id, entry.contentHash);
		if (request !== readRequest) return;
		openingId = null;
		if (response.status === "failed") { error = response.message; return; }
		error = selectKnowledgeArticle(response.data) ?? "";
		linkError = null;
	}

	function startNew() {
		readRequest++;
		openingId = null;
		selected = null;
		draftTitle = "";
		draftSummary = "";
		draftTags = "";
		draftSource = "";
		draftVisibility = "public";
		error = "";
		editing = true;
	}

	function startEdit(document: KnowledgeDocument) {
		draftTitle = document.title;
		draftSummary = document.summary;
		draftTags = document.tags.join(", ");
		draftSource = document.source;
		draftVisibility = document.visibility;
		error = "";
		editing = true;
	}

	function cancelEdit() {
		editing = false;
		error = "";
	}

	function parsedTags() {
		return Array.from(
			new Set(
				draftTags
					.split(",")
					.map((tag) => tag.trim())
					.filter(Boolean),
			),
		);
	}

	async function save() {
		if (saving) return;
		const title = draftTitle.trim();
		const summary = draftSummary.trim();
		const tags = parsedTags();
		if (title === "" || summary === "" || draftSource.trim() === "") {
			error = "Title, summary, and Markdown are required.";
			return;
		}
		if (draftSource.length > 500_000) {
			error = "Article Markdown must be 500,000 characters or fewer.";
			return;
		}
		if (tags.length > 5) {
			error = "Use at most 5 tags.";
			return;
		}
		saving = true;
		error = "";
		const input: KnowledgeDraft = {
			title,
			summary,
			body: draftSource,
			tags,
		};
		const submitted = { title: draftTitle, summary: draftSummary, tags: draftTags, source: draftSource, visibility: draftVisibility };
		const current = selected;
		const response = current === null
			? await oncreate(input)
			: await onupdate(current.id, { ...input, expectedHash: current.contentHash, expectedUpdatedAt: current.updatedAt, visibility: submitted.visibility });
		saving = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		selected = response.data;
		editing = draftTitle !== submitted.title || draftSummary !== submitted.summary || draftTags !== submitted.tags || draftSource !== submitted.source || draftVisibility !== submitted.visibility;
	}
</script>

{#if editing}
	<section class="editor" aria-label="Knowledge editor">
		<header class="page-header">
			<div><h1>{selected ? "Edit knowledge" : "New knowledge"}</h1><p class="page-description">Write and edit your knowledge.</p></div>
			<div class="editor-actions"><button disabled={saving} onclick={cancelEdit}>Cancel</button><button class="save" disabled={saving} onclick={save}>{saving ? "Saving..." : "Save"}</button></div>
		</header>
		{#if error}<p class="editor-error" role="alert">{error}</p>{/if}
		<div class="fields">
			<label>Title<input bind:value={draftTitle} maxlength="240" placeholder="Untitled knowledge" /></label>
			<label>Summary<input bind:value={draftSummary} maxlength="500" placeholder="A short summary" /></label>
		</div>
		<label class="tags">Tags<input bind:value={draftTags} placeholder="rust, api" /></label>
		{#if selected}
            <label class="visibility">Visibility<select aria-label="Visibility" bind:value={draftVisibility} disabled={saving}><option value="public">Public</option><option value="private">Private</option></select></label>
        {/if}
		<label class="markdown">Article body<RichMarkdownEditor bind:value={draftSource} /></label>
	</section>
{:else if selected}
	<section class="reader" id="knowledge-article">
		<KnowledgeHeader title={selected.title} stats={selected.stats}>
			{#snippet actions()}
		<div class="article-actions" aria-label="Article actions">
			{#if selected !== null}<KnowledgeToc entries={selected.toc} content={articleElement} />{/if}
			<button type="button" onclick={() => void copyArticleLink()} aria-label={copiedArticle === selected?.id ? "Article link copied" : "Copy article link"} title={copiedArticle === selected?.id ? "Link copied" : "Copy article link"}>{#if copiedArticle === selected?.id}<Check size={16} />{:else}<Link size={16} />{/if}</button>
			<button type="button" onclick={() => selected !== null && startEdit(selected)} aria-label="Edit article" title="Edit article"><Pencil size={16} /></button>
			<button type="button" onclick={returnToArticle} aria-label={articleTrail.length ? "Back to previous article" : "Back to articles"} title={articleTrail.length ? "Back to previous article" : "Back to articles"}><ArrowLeft size={16} /></button>
		</div>
			{/snippet}
		</KnowledgeHeader>
		{#key selected.id}
			<article data-article-id={selected.id} bind:this={articleElement} use:mediaPlayers={selected.html} class="prose" use:openArticleLinks={{ onError: (message) => { linkError = message; }, onOpen: selectKnowledgeArticle }}>{@html selected.html}</article>
		{/key}
		{#if linkError !== null}<p role="alert">{linkError}</p>{/if}
		{#if copyError !== null}<p role="alert">{copyError}</p>{/if}

	</section>
{:else}
	<section class="index" use:preloadArticles={true}>
		<header class="index-header page-header">
			<div><h1>Knowledge</h1><p class="page-description">Long-form writing, organized by date.</p></div>
			<button onclick={startNew}><Plus size={12} /> New article</button>
		</header>

		{#if error}<p role="alert">{error}</p>{/if}
		{#each groups as group (group.year)}
			<section class="year">
				<h2>{group.year} <small>{group.count} entries</small></h2>
				{#each group.months as month (month.month)}
					<h3>{monthName(month.month)}</h3>
					<ol>
						{#each month.entries as entry (entry.id)}
							<li><time>{new Date(entry.createdAt).getDate().toString().padStart(2, "0")}</time><button data-knowledge-id={entry.id} data-content-hash={entry.contentHash} aria-busy={openingId === entry.id} disabled={openingId === entry.id} onclick={() => void openEntry(entry)}>{entry.title}</button><span>{entry.tags.join(" · ")}</span></li>
						{/each}
					</ol>
				{/each}
			</section>
		{/each}
		{#if groups.length === 0 && !loading}<p class="empty">No knowledge documents found.</p>{/if}
	</section>
{/if}

<style>
	.index { width: 100%; margin: 0 auto; }
	.editor { width: 100%; margin: 0 auto; }
	.editor header { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
	.editor h1 { margin: 0; }
	.editor-actions { display: flex; flex-wrap: wrap; gap: 0.5rem; }
	.editor-actions button { height: 2rem; padding: 0 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); font-size: 0.75rem; }
	.editor-actions .save { border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.editor-actions button:disabled { opacity: 0.5; }
	.fields { display: grid; grid-template-columns: minmax(0, 3fr) minmax(0, 2fr); gap: 1rem; margin-bottom: 1rem; }
	.tags { margin-bottom: 1rem; }
	.editor label { min-width: 0; display: grid; gap: 0.5rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.65rem; letter-spacing: 0.08em; text-transform: uppercase; }
	.editor input, .editor select { min-width: 0; box-sizing: border-box; width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.875rem; letter-spacing: normal; text-transform: none; }
	.editor input, .editor select { height: 2.5rem; padding: 0 0.75rem; }
	.editor input:focus, .editor select:focus { border-color: var(--color-border-strong); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 16%, transparent); }
	.editor-error { padding: 0.75rem; border: 1px solid var(--color-error); border-radius: var(--radius-md); color: var(--color-error); font-size: 0.75rem; }
	.index-header { display: flex; align-items: flex-start; justify-content: space-between; }
	.index-header div { position: relative; }
	.index-header h1 { margin: 0; }
	.index-header button, .article-actions button { display: inline-flex; align-items: center; gap: 0.25rem; border: 0; background: transparent; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.year { margin: 1.25rem 0; }
	.year h2 { margin: 0 0 1rem; }
	.year h2 small { margin-left: 0.25rem; color: var(--color-muted-foreground); font-weight: 400; }
	.year h3 { margin: 0 0 0.5rem; }
	ol { display: grid; gap: 0.375rem; margin: 0 0 1rem; padding: 0; list-style: none; }
	li { display: grid; grid-template-columns: 2.25rem minmax(0, 1fr) max-content; gap: 0.75rem; }
	li time { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.875rem; }
	li button { overflow: hidden; border: 0; background: transparent; color: var(--color-foreground); cursor: pointer; font-size: 0.875rem; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
	li button:hover { color: var(--color-accent); text-decoration: underline; text-underline-offset: 0.25rem; }
	li span { color: var(--color-muted-foreground); font-size: 0.75rem; }
	.empty { padding: 4rem 0; color: var(--color-muted-foreground); font-size: 0.875rem; text-align: center; }
	.reader { position: relative; width: 100%; margin: 0; }
	.prose { min-width: 0; margin-top: 2rem; color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.95rem; line-height: 1.65; overflow-wrap: break-word; word-break: break-word; }
	.prose :global(h1), .prose :global(h2), .prose :global(h3), .prose :global(h4), .prose :global(h5), .prose :global(h6) { position: relative; margin: 2em 0 0.6em; scroll-margin-top: 4rem; }
	.prose :global(h2) { padding-bottom: 0.3em; border-bottom: 1px solid var(--color-border); }
	.prose :global(p), .prose :global(ul:where(:not(.content-article-list))), .prose :global(ol) { margin: 1em 0; }
	.prose :global(ul), .prose :global(ol) { padding-left: 1.6em; }
	.prose :global(ul) { list-style: disc; }
	.prose :global(ol) { list-style: decimal; }
	.prose :global(li) { margin: 0.4em 0; line-height: 1.7; }
	.prose :global(li > ul), .prose :global(li > ol) { margin: 0.25em 0; }
	.prose :global(blockquote) { margin: 1.5em 0; padding: 0.1em 0 0.1em 1.25em; border-left: 2px solid var(--color-muted-foreground); color: var(--color-muted-foreground); font-style: italic; }
	.prose :global(blockquote p) { margin: 0.3em 0; }
	.prose :global(strong) { font-weight: 700; }
	.prose :global(hr) { margin: 2.5em 0; border: 0; border-top: 1px solid var(--color-border); }
	.prose :global(img) { display: block; max-width: 100%; height: auto; margin: 1.5em auto; border-radius: var(--radius-md); }
	.prose :global(a) { color: var(--color-accent); text-decoration: underline; text-decoration-thickness: 1px; text-underline-offset: 0.2em; }
	.prose :global(pre) { max-width: 100%; overflow-x: auto; margin: 1.75em 0; padding: 0.875rem 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-muted); font-family: var(--font-mono); font-size: 0.8125rem; line-height: 1.65; }
	.prose :global(:not(pre) > code) { padding: 0.15em 0.4em; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-muted); font-family: var(--font-mono); font-size: 0.875em; line-height: 1.5; }
	.prose :global(table) { display: block; overflow-x: auto; width: 100%; margin: 1.5em 0; border-collapse: collapse; font-size: 0.9em; }
	.prose :global(th), .prose :global(td) { padding: 0.6rem; border: 1px solid var(--color-border); text-align: left; vertical-align: top; }
	.article-actions { display: flex; flex-shrink: 0; gap: 0.5rem; }

	:global(.page-content[data-stacked="true"]) .editor header { display: grid; }
	:global(.page-content[data-stacked="true"]) .fields { grid-template-columns: minmax(0, 1fr); }
	@media (min-width: 640px) { .prose { font-size: 1rem; line-height: 1.5; } .prose :global(p) { text-align: justify; } }
</style>
