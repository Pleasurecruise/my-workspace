<script lang="ts">
	import { Check, ChevronLeft, ChevronRight, Pencil, Share2, SlidersHorizontal, Trash2, Upload, X } from "@lucide/svelte";
	import { Button, Input, Label, Textarea } from "@my-workspace/ui";
	import { onMount } from "svelte";
	import type { CommandResponse, PhotoItem, PhotoUpdate } from "../consumer";
	import MomentUpload from "./MomentUpload.svelte";
	import R2Image from "./R2Image.svelte";

	let { photos, tags, total, onuploaded, onupdate, ondelete }: { photos: PhotoItem[]; tags: string[]; total: number; onuploaded: (photo: PhotoItem) => void; onupdate: (id: string, input: PhotoUpdate) => Promise<CommandResponse<PhotoItem>>; ondelete: (id: string) => Promise<CommandResponse<string>> } = $props();
	const dateFormatter = new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" });
	let uploadOpen = $state(false);
	let filtersOpen = $state(false);
	let search = $state("");
	let selectedTags = $state<string[]>([]);
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
	let filtered = $derived.by(() => {
		const query = search.trim().toLowerCase();
		return photos
			.filter((photo) => selectedTags.length === 0 || selectedTags.some((tag) => photo.tags.includes(tag)))
			.filter(
				(photo) =>
					query === "" ||
					photo.title.toLowerCase().includes(query) ||
					photo.description?.toLowerCase().includes(query) ||
					photo.tags.some((tag) => tag.includes(query)),
			)
			.sort((left, right) => {
				if (left.date === null && right.date === null) return 0;
				if (left.date === null) return 1;
				if (right.date === null) return -1;
				return oldestFirst ? left.date.localeCompare(right.date) : right.date.localeCompare(left.date);
			});
	});
	let selectedPhoto = $derived(filtered.find((photo) => photo.id === selectedPhotoId) ?? null);
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
		document.body.style.overflow = "hidden";
		return () => {
			document.body.style.overflow = previousOverflow;
		};
	});

	function startEditing(photo: PhotoItem) {
		editingPhotoId = photo.id;
		editTitle = photo.title;
		editDescription = photo.description ?? "";
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

	onMount(() => {
		function onKeydown(event: KeyboardEvent) {
			if (selectedPhoto === null) return;
			if (event.key === "Escape") selectedPhotoId = null;
			if (event.key === "ArrowLeft" && adjacentPhotos.previous !== null) selectedPhotoId = adjacentPhotos.previous.id;
			if (event.key === "ArrowRight" && adjacentPhotos.next !== null) selectedPhotoId = adjacentPhotos.next.id;
		}
		document.addEventListener("keydown", onKeydown);
		return () => document.removeEventListener("keydown", onKeydown);
	});
</script>

<section aria-label="Moment gallery">
	<div class="gallery-heading">
		<div><p>Photo journal</p><h2>Recent moments</h2><span>{filtered.length === photos.length ? `${total} photographs` : `${filtered.length} of ${total} photographs`}</span></div>
		<div class="heading-actions">
			<Button variant={filtersOpen ? "default" : "outline"} size="sm" onclick={() => (filtersOpen = !filtersOpen)}><SlidersHorizontal size={13} /> Filters</Button>
			<Button variant={uploadOpen ? "default" : "outline"} size="sm" onclick={() => (uploadOpen = !uploadOpen)}><Upload size={13} /> Upload</Button>
		</div>
	</div>

	{#if uploadOpen}<MomentUpload {onuploaded} />{/if}
	{#if filtersOpen}
		<div class="filters">
			<Input bind:value={search} placeholder="Search photos…" />
			<div class="tag-index" aria-label="Photo tags">
				{#each tags as tag (tag)}
					<button type="button" class:active={selectedTags.includes(tag)} aria-pressed={selectedTags.includes(tag)} onclick={() => (selectedTags = selectedTags.includes(tag) ? selectedTags.filter((selected) => selected !== tag) : [...selectedTags, tag])}># {tag}</button>
				{/each}
			</div>
			<div class="filter-actions">
				<button type="button" class:active={!oldestFirst} onclick={() => (oldestFirst = false)}>Newest</button>
				<button type="button" class:active={oldestFirst} onclick={() => (oldestFirst = true)}>Oldest</button>
				{#if search !== "" || selectedTags.length > 0}<button type="button" onclick={() => { search = ""; selectedTags = []; }}>Reset</button>{/if}
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
	<div class="viewer" role="dialog" aria-modal="true" aria-label={selectedPhoto.title}>
		<button class="viewer-close" type="button" onclick={() => (selectedPhotoId = null)} aria-label="Close photo"><X size={18} /></button>
		{#if adjacentPhotos.previous !== null}
			{@const previousPhoto = adjacentPhotos.previous}
			<button class="viewer-previous" type="button" onclick={() => (selectedPhotoId = previousPhoto.id)} aria-label="Previous photo"><ChevronLeft size={22} /></button>
		{/if}
		<div class="viewer-image">
			{#key selectedPhoto.r2Key}
				<R2Image objectKey={selectedPhoto.r2Key} thumbHash={selectedPhoto.thumbHash} alt={selectedPhoto.title} width={selectedPhoto.width} height={selectedPhoto.height} />
			{/key}
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
				<div class="viewer-title"><h3>{selectedPhoto.title}</h3><Button variant="ghost" size="sm" onclick={() => startEditing(selectedPhoto)}><Pencil size={13} /> Edit</Button></div>
				{#if selectedPhoto.description}<p>{selectedPhoto.description}</p>{/if}
			{/if}
			{#if mutationError}<p class="mutation-error" role="alert">{mutationError}</p>{/if}
			<Button
				variant="outline"
				size="sm"
				onclick={() =>
					void navigator.clipboard.writeText(`https://moment.you-find.me/photos/${selectedPhoto.id}`).then(
						() => {
							copiedPhotoId = selectedPhoto.id;
							shareError = "";
						},
						() => {
							shareError = "Could not copy the photo link.";
						},
					)}
			>
				{#if copiedPhotoId === selectedPhoto.id}<Check size={13} /> Copied{:else}<Share2 size={13} /> Copy link{/if}
			</Button>
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
				{:else}
					<Button variant="ghost" size="sm" onclick={() => (confirmingDelete = true)}><Trash2 size={13} /> Delete</Button>
				{/if}
			</div>
		</aside>
		{#if adjacentPhotos.next !== null}
			{@const nextPhoto = adjacentPhotos.next}
			<button class="viewer-next" type="button" onclick={() => (selectedPhotoId = nextPhoto.id)} aria-label="Next photo"><ChevronRight size={22} /></button>
		{/if}
	</div>
{/if}

<style>
	.gallery-heading { display: flex; align-items: end; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
	.gallery-heading p, .gallery-heading h2, .gallery-heading span { margin: 0; }
	.gallery-heading p { margin-bottom: 0.35rem; color: var(--color-accent); font-size: 0.72rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	.gallery-heading h2 { font-family: var(--font-serif); font-size: clamp(1.5rem, 3vw, 2.25rem); font-weight: 500; letter-spacing: -0.025em; }
	.gallery-heading span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.72rem; }
	.heading-actions { display: flex; gap: 0.45rem; }
	.filters { display: grid; gap: 0.7rem; margin-bottom: 1rem; padding: 0.8rem; border-block: 1px solid var(--color-divider); }
	.tag-index { display: flex; flex-wrap: wrap; gap: 0.35rem; }
	.tag-index button, .filter-actions button { padding: 0.25rem 0.55rem; border: 1px solid var(--color-border); border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font-size: 0.68rem; }
	.tag-index button.active, .filter-actions button.active { border-color: var(--color-accent); background: color-mix(in srgb, var(--color-accent) 10%, transparent); color: var(--color-accent); }
	.filter-actions { display: flex; gap: 0.35rem; }
	.filter-actions button:last-child { margin-left: auto; }
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
	.viewer-image { display: grid; min-width: 0; place-items: center; padding: 3.5rem; }
	.viewer-image :global(.progressive-image) { width: min(100%, 70rem); max-height: calc(100vh - 7rem); }
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
	.delete-actions { display: flex; align-items: center; justify-content: flex-end; gap: 0.35rem; margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--color-divider); color: var(--color-error); font-size: 0.68rem; }
	.viewer-close, .viewer-previous, .viewer-next { position: fixed; z-index: 51; display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border: 1px solid var(--color-border); border-radius: var(--radius-full); background: var(--color-background); color: var(--color-muted-foreground); cursor: pointer; }
	.viewer-close { top: 1rem; right: 1rem; }
	.viewer-previous, .viewer-next { top: 50%; translate: 0 -50%; }
	.viewer-previous { left: 1rem; }
	.viewer-next { right: 21rem; }
	@media (max-width: 700px) { .masonry { columns: 2 9rem; } .viewer { grid-template-columns: 1fr; overflow-y: auto; } .viewer-image { min-height: 60vh; padding: 3.5rem 2rem 1rem; } .viewer aside { padding: 1rem 1.25rem 2rem; border-top: 1px solid var(--color-divider); border-left: 0; } .viewer-next { right: 1rem; } }
	@media (prefers-reduced-motion: reduce) { figure :global(.progressive-image), figcaption { transition: none; } }
</style>
