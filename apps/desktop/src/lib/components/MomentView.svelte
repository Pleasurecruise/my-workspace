<script lang="ts">
	import { Check, ChevronLeft, ChevronRight, Pencil, Share2, SlidersHorizontal, Trash2, Upload, X } from "@lucide/svelte";
	import { Button, Input, Label, Textarea } from "@my-workspace/ui";
	import { tick } from "svelte";
	import type { CommandResponse, PhotoItem, PhotoUpdate } from "../consumer";
	import MomentUpload from "./MomentUpload.svelte";
	import R2Image from "./R2Image.svelte";

	let { photos, tags, total, onuploaded, onupdate, ondelete }: { photos: PhotoItem[]; tags: string[]; total: number; onuploaded: (photo: PhotoItem) => void; onupdate: (id: string, input: PhotoUpdate) => Promise<CommandResponse<PhotoItem>>; ondelete: (id: string) => Promise<CommandResponse<string>> } = $props();
	const dateFormatter = new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" });
	let uploadOpen = $state(false);
	let filtersOpen = $state(false);
	let selectedTags = $state<string[]>([]);
	let tagMatch = $state<"any" | "all">("any");
	let oldestFirst = $state(false);
	let selectedPhotoId = $state<string | null>(null);
	let copiedPhotoId = $state<string | null>(null);
	let shareError = $state("");
	let editingPhotoId = $state<string | null>(null);
	let editTitle = $state("");
	let editDescription = $state("");
	let editTags = $state("");
	let mutating = $state(false);
	let mutationError = $state("");
	let confirmingDelete = $state(false);
	let viewer = $state<HTMLDivElement | null>(null);
	let closeViewer = $state<HTMLButtonElement | null>(null);
	let filtered = $derived.by(() => {
		return photos
			.filter(
				(photo) =>
					selectedTags.length === 0 ||
					(tagMatch === "all"
						? selectedTags.every((tag) => photo.tags.includes(tag))
						: selectedTags.some((tag) => photo.tags.includes(tag))),
			)
			.sort((left, right) => {
				if (left.date === null && right.date === null) return 0;
				if (left.date === null) return 1;
				if (right.date === null) return -1;
				return oldestFirst ? left.date.localeCompare(right.date) : right.date.localeCompare(left.date);
			});
	});
	let selectedPhoto = $derived.by(() => {
		for (const photo of filtered) {
			if (photo.id === selectedPhotoId) return photo;
		}
		return null;
	});
	let selectedPhotoIndex = $derived(filtered.findIndex((photo) => photo.id === selectedPhotoId));
	let adjacentPhotos = $derived.by(() => {
		let previous: PhotoItem | null = null;
		let selectedFound = false;
		for (const photo of filtered) {
			if (selectedFound) return { previous, next: photo };
			if (photo.id === selectedPhotoId) selectedFound = true;
			else previous = photo;
		}
		return { previous: selectedFound ? previous : null, next: null };
	});

	$effect(() => {
		if (selectedPhotoId !== null && selectedPhoto === null) selectedPhotoId = null;
	});

	$effect(() => {
		const photoId = selectedPhotoId;
		if (photoId === null) return;
		editingPhotoId = null;
		confirmingDelete = false;
		mutationError = "";
		shareError = "";
		const previousOverflow = document.body.style.overflow;
		const previousFocus = document.activeElement;
		document.body.style.overflow = "hidden";
		void tick().then(() => closeViewer?.focus());
		return () => {
			document.body.style.overflow = previousOverflow;
			if (previousFocus instanceof HTMLElement && document.contains(previousFocus)) previousFocus.focus();
		};
	});

	function startEditing(photo: PhotoItem) {
		editingPhotoId = photo.id;
		editTitle = photo.title;
		editDescription = photo.description === null ? "" : photo.description;
		editTags = photo.tags.join(", ");
		mutationError = "";
		confirmingDelete = false;
	}

	async function saveEditing(photo: PhotoItem) {
		const title = editTitle.trim();
		const nextTags = [...new Set(editTags.split(",").map((tag) => tag.trim().toLowerCase()).filter(Boolean))];
		if (title === "") {
			mutationError = "Title is required.";
			return;
		}
		if (nextTags.length > 10 || nextTags.some((tag) => tag.length > 50)) {
			mutationError = "Use at most 10 tags, each no longer than 50 characters.";
			return;
		}
		mutating = true;
		mutationError = "";
		const response = await onupdate(photo.id, {
			title,
			description: editDescription.trim(),
			tags: nextTags,
		});
		mutating = false;
		if (response.status === "failed") {
			mutationError = response.message;
			return;
		}
		editingPhotoId = null;
	}

	async function removePhoto(photo: PhotoItem) {
		mutating = true;
		mutationError = "";
		const response = await ondelete(photo.id);
		mutating = false;
		if (response.status === "failed") {
			mutationError = response.message;
			return;
		}
		selectedPhotoId = null;
		editingPhotoId = null;
		confirmingDelete = false;
	}

	function handleViewerKeydown(event: KeyboardEvent) {
		if (selectedPhoto === null || viewer === null) return;
		if (event.key === "Escape") {
			selectedPhotoId = null;
			return;
		}
		if (event.key === "Tab") {
			const controls = viewer.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), textarea:not(:disabled)");
			if (controls.length === 0) return;
			const first = controls.item(0);
			const last = controls.item(controls.length - 1);
			if (event.shiftKey && document.activeElement === first) {
				event.preventDefault();
				last.focus();
			} else if (!event.shiftKey && document.activeElement === last) {
				event.preventDefault();
				first.focus();
			}
			return;
		}
		const target = event.target;
		if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || (target instanceof HTMLElement && target.isContentEditable)) return;
		if (event.key === "ArrowLeft" && adjacentPhotos.previous !== null) selectedPhotoId = adjacentPhotos.previous.id;
		if (event.key === "ArrowRight" && adjacentPhotos.next !== null) selectedPhotoId = adjacentPhotos.next.id;
	}
</script>

{#if uploadOpen}
	<MomentUpload onuploaded={(photo) => { onuploaded(photo); uploadOpen = false; }} onclose={() => (uploadOpen = false)} />
{:else}
<section aria-label="Moment gallery">
	<div class="gallery-heading">
		<div class="heading-copy">
			<div class="title-row">
				<h2>Gallery</h2>
				<button type="button" class="header-icon" onclick={() => (uploadOpen = true)} aria-label="Upload photos" title="Upload photos"><Upload size={12} /></button>
			</div>
			<span>{filtered.length === photos.length ? `${total} photographs` : `${filtered.length} of ${total} photographs`}</span>
		</div>
		<button type="button" class:active={filtersOpen} class="filter-toggle" onclick={() => (filtersOpen = !filtersOpen)} aria-label="Toggle filters" aria-pressed={filtersOpen} title="Toggle filters"><SlidersHorizontal size={16} /></button>
	</div>

	{#if selectedTags.length > 0}
		<div class="active-filter-chips" aria-label="Active photo filters">
			{#each selectedTags as tag (tag)}
				<span>{tag}<button type="button" onclick={() => (selectedTags = selectedTags.filter((selected) => selected !== tag))} aria-label={`Remove ${tag} filter`}><X size={10} /></button></span>
			{/each}
		</div>
	{/if}
	{#if filtersOpen}
		<div class="filters">
			<div class="filter-grid">
				<section class="tag-filter" aria-labelledby="moment-tag-filter">
					<header>
						<div><h3 id="moment-tag-filter">Filter by tag</h3><p>{tags.length} available</p></div>
						<div class="match-mode" role="group" aria-label="Tag matching mode">
							<button type="button" class:active={tagMatch === "any"} aria-pressed={tagMatch === "any"} onclick={() => (tagMatch = "any")}>Any</button>
							<button type="button" class:active={tagMatch === "all"} aria-pressed={tagMatch === "all"} onclick={() => (tagMatch = "all")}>All</button>
						</div>
					</header>
					<div class="tag-index" aria-label="Photo tags">
						{#each tags as tag (tag)}
							<button type="button" class:active={selectedTags.includes(tag)} aria-pressed={selectedTags.includes(tag)} onclick={() => (selectedTags = selectedTags.includes(tag) ? selectedTags.filter((selected) => selected !== tag) : [...selectedTags, tag])}>{tag}{#if selectedTags.includes(tag)}<Check size={11} />{/if}</button>
						{/each}
					</div>
				</section>
				<section class="date-order" aria-labelledby="moment-date-order">
					<h3 id="moment-date-order">Order by date</h3>
					<div><button type="button" class:active={!oldestFirst} onclick={() => (oldestFirst = false)}>Newest</button><button type="button" class:active={oldestFirst} onclick={() => (oldestFirst = true)}>Oldest</button></div>
					<p>Photos without a date appear at the end.</p>
				</section>
			</div>
			<div class="filter-summary">
				<span>{selectedTags.length} {selectedTags.length === 1 ? "tag" : "tags"} selected</span>
				{#if selectedTags.length > 0 || oldestFirst || tagMatch === "all"}<button type="button" onclick={() => { selectedTags = []; oldestFirst = false; tagMatch = "any"; }}>Reset</button>{/if}
			</div>
		</div>
	{/if}

	<div class="masonry">
		{#each filtered as photo (photo.id)}
			<figure>
				<button type="button" onclick={() => (selectedPhotoId = photo.id)} aria-label={`Open ${photo.title}`}>
					<R2Image objectKey={photo.thumbnailR2Key} thumbHash={photo.thumbHash} alt={photo.title} width={photo.width} height={photo.height} />
				</button>
				<figcaption><strong>{photo.title}</strong>{#if photo.date}<span>{dateFormatter.format(new Date(photo.date))}</span>{/if}<div>{#each photo.tags as tag}<span>#{tag}</span>{/each}</div></figcaption>
			</figure>
		{/each}
	</div>
	{#if filtered.length === 0}<p class="empty">No photos match the current filters.</p>{/if}
</section>

	{#if selectedPhoto !== null}
		<div bind:this={viewer} class="viewer" role="dialog" aria-modal="true" aria-label={selectedPhoto.title} tabindex="-1" onkeydown={handleViewerKeydown}>
			<div class="viewer-main">
				<header class="viewer-header">
					<span>{selectedPhotoIndex + 1} / {filtered.length}</span>
					<div class="viewer-toolbar" aria-label="Photo actions">
						<button type="button" onclick={() => void navigator.clipboard.writeText(`https://moment.you-find.me/photos/${selectedPhoto.id}`).then(() => { copiedPhotoId = selectedPhoto.id; shareError = ""; }, () => { shareError = "Could not copy the photo link."; })} aria-label="Share photo" title={copiedPhotoId === selectedPhoto.id ? "Link copied" : "Share photo"}>{#if copiedPhotoId === selectedPhoto.id}<Check size={15} />{:else}<Share2 size={15} />{/if}</button>
						<button type="button" onclick={() => startEditing(selectedPhoto)} aria-label="Edit photo" title="Edit photo"><Pencil size={15} /></button>
						<button class="danger" type="button" onclick={() => (confirmingDelete = true)} aria-label="Delete photo" title="Delete photo"><Trash2 size={15} /></button>
						<button bind:this={closeViewer} type="button" onclick={() => (selectedPhotoId = null)} aria-label="Close photo" title="Close photo"><X size={18} /></button>
					</div>
				</header>
				{#if adjacentPhotos.previous !== null}
					{@const previousPhoto = adjacentPhotos.previous}
					<button class="viewer-previous" type="button" onclick={() => (selectedPhotoId = previousPhoto.id)} aria-label="Previous photo"><ChevronLeft size={22} /></button>
				{/if}
				<div class="viewer-image">
					{#key selectedPhoto.r2Key}
						<R2Image objectKey={selectedPhoto.r2Key} thumbHash={selectedPhoto.thumbHash} alt={selectedPhoto.title} width={selectedPhoto.width} height={selectedPhoto.height} />
					{/key}
				</div>
				{#if adjacentPhotos.next !== null}
					{@const nextPhoto = adjacentPhotos.next}
					<button class="viewer-next" type="button" onclick={() => (selectedPhotoId = nextPhoto.id)} aria-label="Next photo"><ChevronRight size={22} /></button>
				{/if}
			</div>
			<aside>
			{#if editingPhotoId === selectedPhoto.id}
				<div class="edit-fields">
					<Label>Title<Input bind:value={editTitle} maxlength="120" /></Label>
					<Label>Description<Textarea bind:value={editDescription} maxlength="500" rows="4" /></Label>
					<Label>Tags<Input bind:value={editTags} placeholder="travel, shanghai" /></Label>
					<div class="edit-actions">
						<Button variant="ghost" size="sm" disabled={mutating} onclick={() => (editingPhotoId = null)}>Cancel</Button>
						<Button size="sm" disabled={mutating || editTitle.trim() === ""} onclick={() => void saveEditing(selectedPhoto)}><Check size={13} /> {mutating ? "Saving…" : "Save"}</Button>
					</div>
				</div>
			{:else}
				<div class="viewer-title"><h3>{selectedPhoto.title}</h3></div>
				{#if selectedPhoto.description}<p>{selectedPhoto.description}</p>{/if}
			{/if}
			{#if mutationError}<p class="mutation-error" role="alert">{mutationError}</p>{/if}
			{#if shareError}<p class="share-error" role="alert">{shareError}</p>{/if}
			{#if selectedPhoto.date}<dl><dt>Date</dt><dd>{dateFormatter.format(new Date(selectedPhoto.date))}</dd></dl>{/if}
			{#if selectedPhoto.geo}<dl><dt>Location</dt><dd>{selectedPhoto.geo.lat.toFixed(5)}, {selectedPhoto.geo.lng.toFixed(5)}</dd></dl>{/if}
			<dl><dt>Dimensions</dt><dd>{selectedPhoto.width} × {selectedPhoto.height}</dd></dl>
			{#if selectedPhoto.format}<dl><dt>Format</dt><dd>{selectedPhoto.format}</dd></dl>{/if}
			{#if selectedPhoto.size}<dl><dt>Size</dt><dd>{(selectedPhoto.size / 1024 / 1024).toFixed(1)} MB</dd></dl>{/if}
			{#if selectedPhoto.tags.length > 0}<div class="viewer-tags">{#each selectedPhoto.tags as tag}<span>#{tag}</span>{/each}</div>{/if}
			<div class="delete-actions">
				{#if confirmingDelete}
					<span>Delete this photo and its stored images?</span>
					<Button variant="ghost" size="sm" disabled={mutating} onclick={() => (confirmingDelete = false)}>Cancel</Button>
					<Button variant="destructive" size="sm" disabled={mutating} onclick={() => void removePhoto(selectedPhoto)}>{mutating ? "Deleting…" : "Delete"}</Button>
				{/if}
			</div>
		</aside>
		</div>
{/if}
{/if}

<style>
	.gallery-heading { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
	.heading-copy { min-width: 0; }
	.title-row { display: flex; align-items: center; gap: 0.5rem; }
	.gallery-heading h2, .gallery-heading span { margin: 0; }
	.gallery-heading h2 { font-size: 1.125rem; font-weight: 600; }
	.gallery-heading span { display: block; margin-top: 0.25rem; color: var(--color-muted-foreground); font-size: 0.875rem; }
	.header-icon, .filter-toggle { display: grid; place-items: center; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; transition: color var(--duration-fast), background var(--duration-fast); }
	.header-icon { width: 1.75rem; height: 1.75rem; }
	.filter-toggle { width: 2rem; height: 2rem; }
	.header-icon:hover, .filter-toggle:hover { background: var(--color-muted); color: var(--color-foreground); }
	.filter-toggle.active { background: var(--color-foreground); color: var(--color-background); }
	.active-filter-chips { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1rem; }
	.active-filter-chips > span { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.3rem 0.4rem 0.3rem 0.6rem; border-radius: var(--radius-md); background: var(--color-muted); color: var(--color-foreground); font-size: 0.7rem; }
	.active-filter-chips button { display: grid; width: 1rem; height: 1rem; place-items: center; padding: 0; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.active-filter-chips button:hover { background: var(--color-background); color: var(--color-foreground); }
	.filters { display: grid; gap: 1rem; margin-bottom: 1rem; padding: 1rem 0; border-block: 1px solid var(--color-divider); }
	.filter-grid { display: grid; grid-template-columns: minmax(0, 1fr) 17rem; gap: 2rem; }
	.tag-filter > header { display: flex; align-items: end; justify-content: space-between; gap: 1rem; margin-bottom: 0.75rem; }
	.filters h3, .filters p { margin: 0; }
	.filters h3 { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.62rem; font-weight: 600; letter-spacing: 0.14em; text-transform: uppercase; }
	.filters p { margin-top: 0.25rem; color: var(--color-muted-foreground); font-size: 0.68rem; opacity: 0.72; }
	.match-mode { display: flex; padding: 0.125rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
	.match-mode button, .date-order button, .filter-summary button { border: 0; background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.match-mode button { padding: 0.3rem 0.625rem; border-radius: var(--radius-sm); font-size: 0.68rem; }
	.match-mode button.active { background: var(--color-foreground); color: var(--color-background); }
	.tag-index { display: flex; flex-wrap: wrap; gap: 0.35rem; }
	.tag-index button { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.35rem 0.65rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font-size: 0.7rem; }
	.tag-index button.active { border-color: var(--color-foreground); background: var(--color-foreground); color: var(--color-background); }
	.date-order { padding-left: 2rem; border-left: 1px solid var(--color-divider); }
	.date-order > div { display: grid; grid-template-columns: 1fr 1fr; margin-top: 0.75rem; padding: 0.2rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
	.date-order button { padding: 0.5rem; border-radius: var(--radius-sm); font-size: 0.7rem; }
	.date-order button.active { background: var(--color-muted); color: var(--color-foreground); box-shadow: var(--shadow-xs); }
	.filter-summary { display: flex; align-items: center; justify-content: space-between; padding-top: 0.75rem; border-top: 1px solid var(--color-divider); color: var(--color-muted-foreground); font-size: 0.7rem; }
	.filter-summary button { padding: 0.25rem 0.5rem; border-radius: var(--radius-sm); font-size: inherit; }
	.filter-summary button:hover { background: var(--color-muted); color: var(--color-foreground); }
	.masonry { columns: 3 14rem; column-gap: 0.65rem; }
	figure { position: relative; margin: 0 0 0.65rem; break-inside: avoid; overflow: hidden; border-radius: var(--radius-md); }
	figure > button { display: block; width: 100%; padding: 0; overflow: hidden; border: 0; background: transparent; color: inherit; cursor: zoom-in; text-align: left; }
	figure :global(.progressive-image) { transition: scale var(--duration-slow); }
	figcaption { position: absolute; inset: auto 0 0; display: grid; gap: 0.3rem; padding: 3rem 1rem 0.9rem; background: linear-gradient(transparent, var(--color-image-scrim)); color: var(--color-on-dark); opacity: 0; transition: opacity var(--duration-base); pointer-events: none; }
	figure:hover :global(.progressive-image) { scale: 1.025; }
	figure:hover figcaption, figure:focus-within figcaption { opacity: 1; }
	figcaption strong { font-size: 0.9rem; }
	figcaption > span, figcaption div { font-size: 0.7rem; opacity: 0.78; }
	figcaption div { display: flex; gap: 0.4rem; }
	.empty { padding: 4rem 0; color: var(--color-muted-foreground); text-align: center; }
	.viewer { position: fixed; inset: 0; z-index: 50; display: grid; grid-template-columns: minmax(0, 1fr) minmax(16rem, 20rem); background: color-mix(in srgb, var(--color-background) 96%, transparent); backdrop-filter: blur(0.7rem); }
	.viewer-main { position: relative; display: flex; min-width: 0; min-height: 0; flex-direction: column; }
	.viewer-header { display: flex; height: 3.5rem; flex: 0 0 auto; align-items: center; justify-content: space-between; padding: 0.75rem 1rem; }
	.viewer-header > span { color: var(--color-muted-foreground); font-size: 0.78rem; font-variant-numeric: tabular-nums; }
	.viewer-image { display: grid; min-width: 0; min-height: 0; flex: 1; place-items: center; padding: 1rem 3rem 2rem; }
	.viewer-image :global(.progressive-image) { width: min(100%, 70rem); max-height: calc(100vh - 6.5rem); }
	.viewer aside { padding: 4rem 1.5rem 2rem; overflow-y: auto; border-left: 1px solid var(--color-divider); }
	.viewer h3 { margin: 0 0 1rem; font-family: var(--font-serif); font-size: 1.35rem; }
	.viewer-title { display: flex; align-items: start; justify-content: space-between; gap: 0.5rem; }
	.viewer aside > p { color: var(--color-muted-foreground); font-size: 0.78rem; line-height: 1.6; white-space: pre-wrap; }
	.viewer aside > .share-error { color: var(--color-error); font-size: 0.68rem; }
	.viewer aside > .mutation-error { color: var(--color-error); font-size: 0.68rem; }
	.edit-fields { display: grid; gap: 0.7rem; margin-bottom: 1rem; }
	.edit-fields :global(label) { display: grid; gap: 0.3rem; color: var(--color-muted-foreground); font-size: 0.68rem; }
	.edit-actions { display: flex; justify-content: flex-end; gap: 0.4rem; }
	.viewer dl { display: grid; grid-template-columns: 5rem 1fr; gap: 0.5rem; padding: 0.65rem 0; border-bottom: 1px solid var(--color-divider); font-size: 0.72rem; }
	.viewer dt { color: var(--color-muted-foreground); }
	.viewer dd { margin: 0; }
	.viewer-tags { display: flex; flex-wrap: wrap; gap: 0.35rem; margin-top: 1rem; color: var(--color-accent); font-size: 0.7rem; }
	.delete-actions { display: flex; align-items: center; justify-content: flex-end; gap: 0.35rem; margin-top: 1.5rem; color: var(--color-error); font-size: 0.68rem; }
	.viewer-toolbar { display: flex; align-items: center; gap: 0.5rem; }
	.viewer-toolbar button { display: grid; width: 2rem; height: 2rem; place-items: center; padding: 0; border: 0; border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.viewer-toolbar button:hover { background: var(--color-muted); color: var(--color-foreground); }
	.viewer-toolbar button.danger:hover { background: color-mix(in srgb, var(--color-error) 10%, transparent); color: var(--color-error); }
	.viewer-previous, .viewer-next { position: absolute; z-index: 51; display: grid; width: 2.5rem; height: 2.5rem; place-items: center; border: 0; border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-background) 80%, transparent); color: var(--color-muted-foreground); cursor: pointer; backdrop-filter: blur(8px); }
	.viewer-previous, .viewer-next { top: 50%; translate: 0 -50%; }
	.viewer-previous { left: 1rem; }
	.viewer-next { right: 1rem; }
	.viewer-previous:hover, .viewer-next:hover { background: var(--color-muted); color: var(--color-foreground); }
	@media (max-width: 700px) { .filter-grid { grid-template-columns: 1fr; gap: 1rem; } .date-order { padding: 1rem 0 0; border-top: 1px solid var(--color-divider); border-left: 0; } .masonry { columns: 2 9rem; } .viewer { grid-template-columns: 1fr; overflow-y: auto; } .viewer-main { min-height: 60vh; } .viewer-image { min-height: 0; padding: 1rem 2rem 2rem; } .viewer aside { padding: 1rem 1.25rem 2rem; border-top: 1px solid var(--color-divider); border-left: 0; } }
	@media (prefers-reduced-motion: reduce) { figure :global(.progressive-image), figcaption { transition: none; } }
</style>
