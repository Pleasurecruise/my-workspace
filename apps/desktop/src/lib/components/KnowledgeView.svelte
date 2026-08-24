<script lang="ts">
	import { Library, Pencil, Plus } from "@lucide/svelte";
	import type { CommandResponse, KnowledgeDocument, KnowledgeDraft, KnowledgeUpdate } from "../consumer";
	import KnowledgeHeader from "./KnowledgeHeader.svelte";
	import KnowledgeToc from "./KnowledgeToc.svelte";

	let {
		documents,
		loading,
		oncreate,
		onupdate,
	}: {
		documents: KnowledgeDocument[];
		loading: boolean;
		oncreate: (input: KnowledgeDraft) => Promise<CommandResponse<KnowledgeDocument>>;
		onupdate: (id: string, input: KnowledgeUpdate) => Promise<CommandResponse<KnowledgeDocument>>;
	} = $props();
	let selected = $state<KnowledgeDocument | null>(null);
	let editing = $state(false);
	let draftTitle = $state("");
	let draftSummary = $state("");
	let draftTags = $state("");
	let draftSource = $state("");
	let saving = $state(false);
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
	const newspaperEditionTags = new Set([
		"programmer-daily",
		"developer-daily",
		"personal-daily",
		"newspaper/programmer",
		"newspaper/developer",
		"newspaper/programmer-daily",
		"newspaper/personal",
		"newspaper/personal-daily",
		"程序员日报",
		"个人日报",
	]);

	let groups = $derived.by(() => {
		const years: KnowledgeYear[] = [];
		for (const document of documents) {
			const tags = new Set(document.tags.map((tag) => tag.trim().toLowerCase()));
			if (
				tags.has("newspaper/daily") ||
				(tags.has("newspaper") && tags.has("daily")) ||
				[...tags].some((tag) => newspaperEditionTags.has(tag))
			) continue;
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
		draftTags = "";
		draftSource = "";
		error = "";
		editing = true;
	}

	function startEdit(document: KnowledgeDocument) {
		draftTitle = document.title;
		draftSummary = document.summary;
		draftTags = document.tags.join(", ");
		draftSource = document.source;
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
		const current = selected;
		const response = current === null
			? await oncreate(input)
			: await onupdate(current.id, { ...input, expectedHash: current.contentHash });
		saving = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		selected = response.data;
		editing = false;
	}
</script>

{#if editing}
	<section class="editor" aria-label="Knowledge editor">
		<header>
			<div><h1>{selected ? "Edit knowledge" : "New knowledge"}</h1><p>Markdown is compiled by Rust and saved through my-knowledge.</p></div>
			<div class="editor-actions"><button disabled={saving} onclick={cancelEdit}>Cancel</button><button class="save" disabled={saving} onclick={save}>{saving ? "Saving..." : "Save"}</button></div>
		</header>
		{#if error}<p class="editor-error" role="alert">{error}</p>{/if}
		<div class="fields">
			<label>Title<input bind:value={draftTitle} maxlength="240" placeholder="Untitled knowledge" /></label>
			<label>Summary<input bind:value={draftSummary} maxlength="500" placeholder="A short summary" /></label>
		</div>
		<label class="tags">Tags<input bind:value={draftTags} placeholder="rust, api" /></label>
		<label class="markdown">Markdown<textarea bind:value={draftSource} maxlength="500000" placeholder="Start writing..." spellcheck="true"></textarea></label>
	</section>
{:else if selected}
	<section class="reader" id="knowledge-article">
		<KnowledgeHeader title={selected.title} text={selected.source} />
		<KnowledgeToc entries={selected.toc} />
		<article class="prose">{@html selected.html}</article>
		<aside class="article-actions" aria-label="Article actions">
			<button type="button" onclick={() => (selected = null)} aria-label="All knowledge" title="All knowledge"><Library size={16} /></button>
			<button type="button" onclick={() => selected !== null && startEdit(selected)} aria-label="Edit article" title="Edit article"><Pencil size={16} /></button>
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
		{#if groups.length === 0 && !loading}<p class="empty">No knowledge documents found.</p>{/if}
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
	.editor-actions .save { border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.editor-actions button:disabled { opacity: 0.5; }
	.fields { display: grid; grid-template-columns: 3fr 2fr; gap: 1rem; margin-bottom: 1rem; }
	.tags { margin-bottom: 1rem; }
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
	.index-header button, .article-actions button { display: inline-flex; align-items: center; gap: 0.25rem; border: 0; background: transparent; color: var(--color-muted-foreground); font-size: 0.75rem; }
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
	.reader { position: relative; width: min(100%, 40.625rem); margin: 2rem auto 0; }
	.prose { min-width: 0; margin-top: 2rem; color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.95rem; line-height: 1.65; overflow-wrap: break-word; word-break: break-word; }
	.prose :global(h1), .prose :global(h2), .prose :global(h3), .prose :global(h4), .prose :global(h5), .prose :global(h6) { position: relative; margin: 2em 0 0.6em; font-family: var(--font-sans); font-weight: 600; letter-spacing: normal; line-height: 1.45; scroll-margin-top: 4rem; }
	.prose :global(h1) { font-size: 1.55em; }
	.prose :global(h2) { padding-bottom: 0.3em; border-bottom: 1px solid var(--color-border); font-size: 1.25em; }
	.prose :global(h3) { font-size: 1.12em; }
	.prose :global(h4) { font-size: 1.05em; }
	.prose :global(h5) { font-size: 0.95em; }
	.prose :global(h6) { color: var(--color-muted-foreground); font-size: 0.875em; }
	.prose :global(p), .prose :global(ul), .prose :global(ol) { margin: 1em 0; }
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
	.article-actions { position: fixed; top: 50%; z-index: 20; display: grid; gap: 0.25rem; width: 2.5rem; margin-left: calc(40.625rem + 2rem); translate: 0 -50%; }
	.article-actions button { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: var(--radius-full); cursor: pointer; }
	.article-actions button:hover { background: var(--color-muted); color: var(--color-foreground); }
	@media (max-width: 1180px) {
		.article-actions {
			top: auto;
			right: 1.5rem;
			bottom: 4.75rem;
			display: grid;
			width: auto;
			margin-left: 0;
			translate: 0;
		}
		.article-actions button {
			border: 1px solid var(--color-border);
			background: color-mix(in srgb, var(--color-background) 90%, transparent);
			box-shadow: var(--shadow-xs);
			backdrop-filter: blur(12px);
		}
	}
	@media (max-width: 640px) { .editor header { display: grid; } .fields { grid-template-columns: 1fr; } }
	@media (max-width: 700px) { .article-actions { right: 1rem; bottom: 4.5rem; } }
	@media (min-width: 640px) { .prose { font-size: 1rem; line-height: 1.5; } .prose :global(p) { text-align: justify; } }
</style>
