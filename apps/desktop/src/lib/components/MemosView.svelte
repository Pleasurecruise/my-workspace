<script lang="ts">
	import { Archive, Check, Globe, Heart, Lock, Pencil, Share2, Star, Trash2, X } from "@lucide/svelte";
	import { Alert, AlertDescription, Badge, Button, Input } from "@my-workspace/ui";
	import type { CommandResponse, MemoUpdateInput, MemoView } from "../consumer";
	import MemoEditor from "./MemoEditor.svelte";

	let {
		memos,
		oncreate,
		onupdate,
		ondelete,
	}: {
		memos: MemoView[];
		oncreate: (content: string, visibility: "public" | "private") => Promise<CommandResponse<MemoView>>;
		onupdate: (id: string, input: MemoUpdateInput) => Promise<CommandResponse<MemoView>>;
		ondelete: (id: string) => Promise<CommandResponse<string>>;
	} = $props();

	const relativeFormatter = new Intl.RelativeTimeFormat("en", { numeric: "auto" });
	const dateFormatter = new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric" });
	let draft = $state("");
	let visibility = $state<"public" | "private">("private");
	let search = $state("");
	let saving = $state(false);
	let error = $state("");
	let editingId = $state<string | null>(null);
	let editContent = $state("");
	let editVisibility = $state<"public" | "private">("private");
	let savedContent = "";
	let savedVisibility: "public" | "private" = "private";
	let updating = $state(false);
	let mutatingId = $state<string | null>(null);
	let sharedId = $state<string | null>(null);
	let confirmingDelete = $state<string | null>(null);
	let deleting = $state(false);

	let filtered = $derived(
		memos.filter(
			(memo) =>
				!memo.archived &&
				(memo.content.toLowerCase().includes(search.toLowerCase()) ||
					memo.tags.some((tag) => tag.includes(search.toLowerCase()))),
		),
	);

	function relativeTime(value: string) {
		const seconds = Math.round((Date.parse(value) - Date.now()) / 1_000);
		const absolute = Math.abs(seconds);
		if (absolute < 60) return relativeFormatter.format(seconds, "second");
		if (absolute < 3_600) return relativeFormatter.format(Math.round(seconds / 60), "minute");
		if (absolute < 86_400) return relativeFormatter.format(Math.round(seconds / 3_600), "hour");
		if (absolute < 2_592_000) return relativeFormatter.format(Math.round(seconds / 86_400), "day");
		if (absolute < 31_536_000)
			return relativeFormatter.format(Math.round(seconds / 2_592_000), "month");
		return relativeFormatter.format(Math.round(seconds / 31_536_000), "year");
	}

	function dateBadge(value: string) {
		const date = new Date(value);
		const today = new Date();
		if (
			date.getFullYear() === today.getFullYear() &&
			date.getMonth() === today.getMonth() &&
			date.getDate() === today.getDate()
		)
			return "Today";
		const yesterday = new Date(today);
		yesterday.setDate(today.getDate() - 1);
		if (
			date.getFullYear() === yesterday.getFullYear() &&
			date.getMonth() === yesterday.getMonth() &&
			date.getDate() === yesterday.getDate()
		)
			return "Yesterday";
		return dateFormatter.format(date);
	}

	async function saveDraft() {
		if (saving || draft.trim() === "") return;
		saving = true;
		error = "";
		const response = await oncreate(draft, visibility);
		saving = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		draft = "";
	}

	async function startEdit(memo: MemoView) {
		if (editingId === memo.id || updating) return;
		if (editingId !== null && !(await saveEdit(editingId))) return;
		editingId = memo.id;
		editContent = memo.content;
		editVisibility = memo.visibility;
		savedContent = memo.content;
		savedVisibility = memo.visibility;
		error = "";
	}

	function cancelEdit() {
		if (updating) return;
		editingId = null;
		editContent = "";
		savedContent = "";
	}

	async function saveEdit(id: string): Promise<boolean> {
		if (editingId !== id || editContent.trim() === "" || updating) return false;
		if (editContent === savedContent && editVisibility === savedVisibility) {
			cancelEdit();
			return true;
		}
		updating = true;
		error = "";
		const response = await onupdate(id, { content: editContent, visibility: editVisibility });
		updating = false;
		if (response.status === "failed") {
			error = response.message;
			return false;
		}
		cancelEdit();
		return true;
	}

	async function togglePin(memo: MemoView) {
		if (mutatingId !== null) return;
		mutatingId = memo.id;
		error = "";
		const response = await onupdate(memo.id, { pinned: !memo.pinned });
		mutatingId = null;
		if (response.status === "failed") error = response.message;
	}

	async function toggleFavorite(memo: MemoView) {
		if (mutatingId !== null) return;
		mutatingId = memo.id;
		error = "";
		const response = await onupdate(memo.id, { favorite: !memo.favorite });
		mutatingId = null;
		if (response.status === "failed") error = response.message;
	}

	async function archive(memo: MemoView) {
		if (mutatingId !== null) return;
		mutatingId = memo.id;
		error = "";
		const response = await onupdate(memo.id, { archived: true });
		mutatingId = null;
		if (response.status === "failed") error = response.message;
	}

	function share(memo: MemoView) {
		void navigator.clipboard
			.writeText(`https://memos.you-find.me/memo/${memo.id}`)
			.then(() => {
				sharedId = memo.id;
			})
			.catch(() => {
				error = "Could not copy the memo link.";
			});
	}

	async function remove(memo: MemoView) {
		if (deleting) return;
		deleting = true;
		error = "";
		const response = await ondelete(memo.id);
		deleting = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		confirmingDelete = null;
	}
</script>

<section class="home" aria-label="Memo feed">
	<div class="composer">
		<MemoEditor
			bind:value={draft}
			placeholder="What's on your mind? Markdown is supported."
			onsubmit={saveDraft}
		/>
		<div class="composer-toolbar">
			<Button variant="outline" size="sm" class="gap-1.5 font-normal text-muted-foreground" onclick={() => (visibility = visibility === "public" ? "private" : "public")}>
				{#if visibility === "public"}<Globe size={11} /> Public{:else}<Lock size={11} /> Private{/if}
			</Button>
			<span class="shortcut">⌘ Enter</span>
			<Button size="sm" disabled={saving || draft.trim() === ""} onclick={saveDraft}>
				{saving ? "Saving..." : "Save"}
			</Button>
		</div>
	</div>

	{#if error}
		<Alert class="mb-5 flex items-center gap-3" variant="error">
			<AlertDescription class="flex-1 text-xs">{error}</AlertDescription>
			<button class="border-0 bg-transparent text-inherit" type="button" onclick={() => (error = "")} aria-label="Dismiss error">×</button>
		</Alert>
	{/if}

	<label class="search">
		<span aria-hidden="true">⌕</span>
		<Input class="h-10 px-10 text-sm focus-visible:border-accent" bind:value={search} placeholder="Search memos..." />
		{#if search}
			<button type="button" onclick={() => (search = "")} aria-label="Clear search">×</button>
		{/if}
	</label>

	<div class="memo-list">
		{#each filtered as memo (memo.id)}
			<article id="memo-{memo.id}" class:editing={editingId === memo.id}>
				<header>
					<time datetime={memo.createdAt}>{relativeTime(memo.createdAt)}</time>
					{#if memo.metadataComplete}
						<span class="dot">·</span>
						<span class="visibility-label">
							{#if memo.visibility === "public"}<Globe size={11} /> Public{:else}<Lock size={11} /> Private{/if}
						</span>
					{/if}
					<Badge class="ml-auto bg-foreground text-[0.68rem] text-background">{dateBadge(memo.createdAt)}</Badge>
				</header>

				{#if editingId === memo.id}
					<div class="inline-editor">
						<MemoEditor bind:value={editContent} placeholder="Edit memo..." onsubmit={() => saveEdit(memo.id)} />
					</div>
				{:else}
					<div class="memo-content">{@html memo.html}</div>
				{/if}

				{#if memo.tags.length > 0 && editingId !== memo.id}
					<div class="tags">
						{#each memo.tags as tag (tag)}<button type="button" onclick={() => (search = tag)}><Badge variant="outline" class="border-accent/25 text-accent hover:bg-accent/8">#{tag}</Badge></button>{/each}
					</div>
				{/if}

				<footer>
					{#if editingId === memo.id}
						<Button variant="ghost" size="sm" class="gap-1.5 font-normal text-muted-foreground" onclick={cancelEdit}><X size={12} /> Cancel</Button>
						<Button variant="outline" size="sm" class="gap-1.5 font-normal text-muted-foreground" onclick={() => (editVisibility = editVisibility === "public" ? "private" : "public")}>
							{#if editVisibility === "public"}<Globe size={11} /> Public{:else}<Lock size={11} /> Private{/if}
						</Button>
						<Button
							size="sm"
							class="ml-auto gap-1.5 font-normal"
							disabled={updating || editContent.trim() === ""}
							onclick={() => saveEdit(memo.id)}
						>
							<Check size={12} /> {updating ? "Saving..." : "Save"}
						</Button>
					{:else}
						{#if confirmingDelete === memo.id}
							<span class="delete-confirmation">Permanently delete?</span>
							<Button variant="ghost" size="sm" class="font-normal text-muted-foreground" onclick={() => (confirmingDelete = null)}>Cancel</Button>
							<Button variant="destructive" size="sm" class="font-normal" disabled={deleting} onclick={() => remove(memo)}>
								{deleting ? "Deleting..." : "Delete"}
							</Button>
						{:else}
							<Button variant="ghost" size="sm" class={memo.pinned ? "gap-1.5 font-normal text-accent" : "gap-1.5 font-normal text-muted-foreground"} disabled={mutatingId === memo.id} onclick={() => togglePin(memo)} title={memo.pinned ? "Unpin memo" : "Pin memo"}><Star size={12} fill={memo.pinned ? "currentColor" : "none"} />{memo.pinned ? "Unpin" : "Pin"}</Button>
							<Button variant="ghost" size="sm" class={memo.favorite ? "gap-1.5 font-normal text-accent" : "gap-1.5 font-normal text-muted-foreground"} disabled={mutatingId === memo.id} onclick={() => toggleFavorite(memo)} title={memo.favorite ? "Unfavorite memo" : "Favorite memo"}><Heart size={12} fill={memo.favorite ? "currentColor" : "none"} />{memo.favorite ? "Unfavorite" : "Favorite"}</Button>
							<Button variant="ghost" size="sm" class="gap-1.5 font-normal text-muted-foreground" disabled={updating} onclick={() => startEdit(memo)}><Pencil size={12} /> Edit</Button>
							<Button variant="ghost" size="sm" class="gap-1.5 font-normal text-muted-foreground" disabled={mutatingId === memo.id} onclick={() => archive(memo)}><Archive size={12} />{mutatingId === memo.id ? "Archiving…" : "Archive"}</Button>
							<Button variant="ghost" size="sm" class="gap-1.5 font-normal text-muted-foreground" onclick={() => share(memo)}><Share2 size={12} />{sharedId === memo.id ? "Copied" : memo.visibility === "public" ? "Share" : "Copy link"}</Button>
							<Button variant="destructive" size="sm" class="ml-auto gap-1.5 font-normal" onclick={() => (confirmingDelete = memo.id)}><Trash2 size={12} /> Delete</Button>
						{/if}
					{/if}
				</footer>
			</article>
		{/each}

		{#if filtered.length === 0}
			<p class="empty">No memos found.</p>
		{/if}
	</div>
</section>

<style>
	.home {
		width: min(100%, 42rem);
		margin: 0 auto;
	}

	.composer,
	article {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
	}

	.composer {
		overflow: hidden;
		margin-bottom: 1.25rem;
	}

	.composer:focus-within,
	article.editing:focus-within {
		border-color: var(--color-accent);
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 16%, transparent);
	}

	.composer-toolbar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 0.75rem;
		border-top: 1px solid var(--color-border);
	}

	.shortcut {
		margin-left: auto;
		color: var(--color-muted-foreground);
		font-size: 0.7rem;
		opacity: 0.5;
	}

	.search {
		position: relative;
		display: block;
		margin-bottom: 1.25rem;
	}

	.search > span {
		position: absolute;
		top: 50%;
		left: 0.75rem;
		translate: 0 -50%;
		color: var(--color-muted-foreground);
		font-size: 1rem;
		pointer-events: none;
	}

	.search button {
		position: absolute;
		top: 50%;
		right: 0.5rem;
		width: 1.5rem;
		height: 1.5rem;
		translate: 0 -50%;
		border: 0;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
		font-family: inherit;
	}

	.memo-list {
		display: grid;
		gap: 0.75rem;
	}

	article {
		padding: 1rem 1.25rem;
		transition:
			border-color var(--duration-fast),
			box-shadow var(--duration-fast);
	}

	article:hover {
		border-color: var(--color-border-strong);
		box-shadow: var(--shadow-sm);
	}

	article header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
		color: var(--color-muted-foreground);
		font-size: 0.75rem;
	}

	.dot {
		opacity: 0.4;
	}

	.visibility-label {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	.memo-content {
		max-height: 12rem;
		overflow: auto;
		color: var(--color-foreground);
		font-size: 0.875rem;
		line-height: 1.625;
		overflow-wrap: anywhere;
	}

	.memo-content :global(p:first-child) {
		margin-top: 0;
	}

	.memo-content :global(p:last-child) {
		margin-bottom: 0;
	}

	.memo-content :global(p + p),
	.memo-content :global(h1),
	.memo-content :global(h2),
	.memo-content :global(h3),
	.memo-content :global(ul),
	.memo-content :global(ol),
	.memo-content :global(blockquote),
	.memo-content :global(pre),
	.memo-content :global(table) {
		margin-top: 0.75rem;
	}

	.memo-content :global(h1),
	.memo-content :global(h2),
	.memo-content :global(h3) {
		font-weight: 600;
		line-height: 1.35;
	}

	.memo-content :global(ul),
	.memo-content :global(ol) {
		padding-left: 1.25rem;
	}

	.memo-content :global(a) {
		color: var(--color-accent);
		text-decoration: underline;
		text-underline-offset: 0.18em;
	}

	.memo-content :global(img) {
		display: block;
		max-width: 100%;
		max-height: 32rem;
		height: auto;
		margin: 0.75rem 0;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		object-fit: contain;
	}

	.memo-content :global(code) {
		padding: 0.1rem 0.35rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: color-mix(in srgb, var(--color-muted) 45%, transparent);
		font-family: var(--font-mono);
		font-size: 0.875em;
	}

	.memo-content :global(pre) {
		overflow-x: auto;
		padding: 0.875rem 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--color-muted) 50%, transparent);
	}

	.memo-content :global(pre code) {
		padding: 0;
		border: 0;
		background: transparent;
	}

	.memo-content :global(table) {
		width: 100%;
		border-collapse: collapse;
	}

	.memo-content :global(th),
	.memo-content :global(td) {
		padding: 0.45rem 0.6rem;
		border: 1px solid var(--color-border);
		text-align: left;
	}

	.memo-content :global(blockquote) {
		padding-left: 0.875rem;
		border-left: 3px solid color-mix(in srgb, var(--color-accent) 45%, var(--color-border));
		color: var(--color-muted-foreground);
	}

	.inline-editor {
		margin: -0.25rem -0.75rem;
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-top: 0.75rem;
	}

	.tags button {
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
		font-family: inherit;
	}

	article footer {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.625rem;
		padding-top: 0.625rem;
		border-top: 1px solid var(--color-border);
		overflow-x: auto;
		opacity: 0;
		scrollbar-width: none;
		transition: opacity var(--duration-fast);
	}

	article footer::-webkit-scrollbar { display: none; }

	article:hover footer,
	article:focus-within footer {
		opacity: 1;
	}

	.delete-confirmation {
		margin-right: auto;
		color: var(--color-error);
		font-size: 0.75rem;
	}

	.empty {
		padding: 4rem 0;
		color: var(--color-muted-foreground);
		font-size: 0.875rem;
		text-align: center;
	}

	@media (max-width: 700px) {
		.shortcut {
			display: none;
		}

		article footer {
			opacity: 1;
		}
	}
</style>
