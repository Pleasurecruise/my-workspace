<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { ArrowLeft, Upload, X } from "@lucide/svelte";
	import { Button, Input, Label, Textarea } from "@my-workspace/ui";
	import { onDestroy } from "svelte";
	import type { CommandResponse, PhotoItem, PhotoUpload } from "../consumer";

	let { onuploaded, onclose }: { onuploaded: (photo: PhotoItem) => void; onclose: () => void } = $props();

	let file = $state<File | null>(null);
	let preview = $state<string | null>(null);
	let previewFailed = $state(false);
	let title = $state("");
	let description = $state("");
	let tagText = $state("");
	let date = $state("");
	let latitude = $state("");
	let longitude = $state("");
	let uploading = $state(false);
	let dragging = $state(false);
	let error = $state("");
	let tags = $derived([...new Set(tagText.split(",").map((tag) => tag.trim().toLowerCase()).filter(Boolean))]);
	const sourceLimit = 20 * 1024 * 1024;
	const acceptedTypes = new Set(["image/png", "image/jpeg", "image/webp", "image/avif", "image/heic", "image/heif"]);
	const acceptedExtensions = [".png", ".jpg", ".jpeg", ".webp", ".avif", ".heic", ".heif"];

	function selectFile(selected: File) {
		const name = selected.name.toLowerCase();
		if (!acceptedTypes.has(selected.type) && !acceptedExtensions.some((extension) => name.endsWith(extension))) {
			error = "Please select a PNG, JPEG, WebP, AVIF, or HEIC image.";
			return;
		}
		if (selected.size > sourceLimit) {
			error = "The photo exceeds the 20 MB limit.";
			return;
		}
		if (preview !== null) URL.revokeObjectURL(preview);
		file = selected;
		preview = URL.createObjectURL(selected);
		previewFailed = false;
		title = selected.name.replace(/\.[^.]+$/, "");
		error = "";
	}

	function selectInputFile(input: HTMLInputElement) {
		const selected = input.files === null ? null : input.files.item(0);
		input.value = "";
		if (selected !== null) selectFile(selected);
	}

	function dragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "copy";
		dragging = true;
	}

	function dragLeave(event: DragEvent) {
		if (event.currentTarget instanceof HTMLElement && event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
		dragging = false;
	}

	function dropFile(event: DragEvent) {
		event.preventDefault();
		dragging = false;
		const selected = event.dataTransfer === null ? null : event.dataTransfer.files.item(0);
		if (selected !== null) selectFile(selected);
	}

	function clearFile() {
		if (preview !== null) URL.revokeObjectURL(preview);
		file = null;
		preview = null;
		previewFailed = false;
		title = "";
		description = "";
		tagText = "";
		date = "";
		latitude = "";
		longitude = "";
		error = "";
	}

	async function publish() {
		if (file === null || uploading || title.trim() === "") return;
		if (tags.length > 10 || tags.some((tag) => tag.length > 50)) {
			error = "Use at most 10 tags, each no longer than 50 characters.";
			return;
		}
		const hasLatitude = latitude.trim() !== "";
		const hasLongitude = longitude.trim() !== "";
		if (hasLatitude !== hasLongitude) {
			error = "Latitude and longitude must be provided together.";
			return;
		}
		const lat = Number(latitude);
		const lng = Number(longitude);
		if (hasLatitude && (!Number.isFinite(lat) || !Number.isFinite(lng) || lat < -90 || lat > 90 || lng < -180 || lng > 180)) {
			error = "Coordinates are outside the valid latitude or longitude range.";
			return;
		}
		const selected = file;
		uploading = true;
		error = "";
		let source: number[];
		try {
			source = Array.from(new Uint8Array(await selected.arrayBuffer()));
		} catch {
			error = "The photo could not be read.";
			uploading = false;
			return;
		}
		const input: PhotoUpload = {
			title: title.trim(),
			description: description.trim() === "" ? null : description.trim(),
			tags,
			date: date === "" ? null : new Date(date).toISOString(),
			geo: hasLatitude ? { lat, lng } : null,
		};
		const response = await invoke<CommandResponse<PhotoItem>>("create_photo", { input, source });
		uploading = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		onuploaded(response.data);
		clearFile();
	}

	onDestroy(() => {
		if (preview !== null) URL.revokeObjectURL(preview);
	});
</script>

<section class="uploader" aria-label="Upload a Moment photo">
	<header>
		<div class="upload-title">
			<button type="button" disabled={uploading} onclick={onclose} aria-label="Back to gallery" title="Back to gallery"><ArrowLeft size={16} /></button>
			<div><Upload size={18} /><strong>Upload Photo</strong></div>
		</div>
		{#if file !== null}<Button size="sm" disabled={uploading || title.trim() === ""} onclick={publish}>{uploading ? "Publishing…" : "Publish"}</Button>{/if}
	</header>

	{#if file === null}
		<label class:dragging class="drop-zone" ondragover={dragOver} ondragleave={dragLeave} ondrop={dropFile}>
			<Upload size={22} />
			<span>{dragging ? "Release to select this photo" : "Click or drag to select a photo"}</span>
			<small>PNG, JPEG, WebP, AVIF or HEIC · maximum 20 MB</small>
			<input
				type="file"
				accept="image/png,image/jpeg,image/webp,image/avif,image/heic,image/heif,.heic,.heif"
				onchange={(event) => selectInputFile(event.currentTarget)}
			/>
		</label>
	{:else}
		<div class="upload-grid">
			<div class="preview">
				{#if preview !== null && !previewFailed}<img src={preview} alt="Selected upload preview" onerror={() => (previewFailed = true)} />{:else}<span>{file.name}</span>{/if}
				<button type="button" disabled={uploading} onclick={clearFile} aria-label="Remove selected image"><X size={14} /></button>
			</div>
			<div class="fields">
				<Label>Title<Input bind:value={title} maxlength="120" /></Label>
				<Label>Description<Textarea bind:value={description} maxlength="500" rows="2" /></Label>
				<Label>Tags<Input bind:value={tagText} placeholder="travel, shanghai" /></Label>
				{#if tags.length > 0}
					<div class="upload-tags" aria-label="Tags to publish">
						{#each tags as tag (tag)}<span>#{tag}</span>{/each}
					</div>
				{/if}
				<Label>Date<Input bind:value={date} type="datetime-local" /></Label>
				<div class="coordinates">
					<Label>Latitude<Input bind:value={latitude} type="number" min="-90" max="90" step="0.000001" /></Label>
					<Label>Longitude<Input bind:value={longitude} type="number" min="-180" max="180" step="0.000001" /></Label>
				</div>
			</div>
		</div>
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.uploader { width: min(100%, 42rem); margin: 0 auto; }
	header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
	.upload-title, .upload-title div { display: flex; align-items: center; gap: 0.75rem; }
	.upload-title div { gap: 0.5rem; }
	.upload-title button { display: grid; width: 2rem; height: 2rem; place-items: center; padding: 0; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.upload-title button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-foreground); }
	.upload-title button:disabled { cursor: not-allowed; opacity: 0.5; }
	.upload-title strong { font-size: 1.125rem; font-weight: 600; }
	.upload-title :global(svg) { color: var(--color-muted-foreground); }
	.drop-zone { display: grid; min-height: 15rem; place-items: center; align-content: center; gap: 0.5rem; padding: 2.5rem; border: 2px dashed var(--color-border); border-radius: var(--radius-lg); color: var(--color-muted-foreground); cursor: pointer; transition: border-color var(--duration-fast), background var(--duration-fast); }
	.drop-zone:hover, .drop-zone.dragging { border-color: color-mix(in srgb, var(--color-accent) 50%, var(--color-border)); background: color-mix(in srgb, var(--color-accent) 5%, transparent); color: var(--color-foreground); }
	.drop-zone.dragging { transform: scale(1.01); }
	.drop-zone span { font-size: 0.875rem; font-weight: 600; }
	.drop-zone small { font-size: 0.75rem; }
	.drop-zone input { position: absolute; width: 1px; height: 1px; overflow: hidden; opacity: 0; }
	.upload-grid { display: grid; gap: 1.25rem; }
	.preview { position: relative; min-height: 12rem; max-height: 24rem; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-muted); }
	.preview img { width: 100%; height: 100%; object-fit: contain; }
	.preview button { position: absolute; top: 0.5rem; right: 0.5rem; display: grid; width: 1.8rem; height: 1.8rem; place-items: center; border: 0; border-radius: var(--radius-full); background: var(--color-image-scrim); color: var(--color-on-dark); cursor: pointer; }
	.fields { display: grid; gap: 0.65rem; }
	.fields :global(label) { display: grid; gap: 0.3rem; color: var(--color-muted-foreground); font-size: 0.68rem; }
	.upload-tags { display: flex; flex-wrap: wrap; gap: 0.35rem; color: var(--color-accent); font-size: 0.68rem; }
	.upload-tags span { padding: 0.2rem 0.45rem; border: 1px solid color-mix(in srgb, var(--color-accent) 25%, transparent); border-radius: var(--radius-full); }
	.coordinates { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem; }
	.error { margin: 1rem 0 0; color: var(--color-error); font-size: 0.75rem; }
</style>
