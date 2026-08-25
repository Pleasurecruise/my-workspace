<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { ArrowLeft, Upload, X } from "@lucide/svelte";
	import { Button, Input, Label, Textarea } from "@my-workspace/ui";
	import { onDestroy } from "svelte";
	import { rgbaToThumbHash } from "thumbhash";
	import type { CommandResponse, PhotoItem, PhotoUpload } from "../consumer";

	let { onuploaded, onclose }: { onuploaded: (photo: PhotoItem) => void; onclose: () => void } = $props();

	let file = $state<File | null>(null);
	let preview = $state<string | null>(null);
	let title = $state("");
	let description = $state("");
	let tagText = $state("");
	let date = $state("");
	let latitude = $state("");
	let longitude = $state("");
	let uploading = $state(false);
	let error = $state("");
	let tags = $derived([...new Set(tagText.split(",").map((tag) => tag.trim().toLowerCase()).filter(Boolean))]);

	function selectFile(input: HTMLInputElement) {
		const files = input.files;
		input.value = "";
		if (files === null) return;
		const selected = files.item(0);
		if (selected === null) return;
		if (!["image/png", "image/jpeg", "image/webp", "image/avif"].includes(selected.type.toLowerCase())) {
			error = `Unsupported image MIME type: ${selected.type}.`;
			return;
		}
		if (selected.size > 20 * 1024 * 1024) {
			error = "The image exceeds the 20 MB upload limit.";
			return;
		}
		if (preview !== null) URL.revokeObjectURL(preview);
		file = selected;
		preview = URL.createObjectURL(selected);
		title = selected.name.replace(/\.[^.]+$/, "");
		error = "";
	}

	function clearFile() {
		if (preview !== null) URL.revokeObjectURL(preview);
		file = null;
		preview = null;
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
		let bitmap: ImageBitmap | null = null;
		try {
			bitmap = await createImageBitmap(selected);
				const originalCanvas = new OffscreenCanvas(bitmap.width, bitmap.height);
				const originalContext = originalCanvas.getContext("2d");
				if (originalContext === null) {
					error = "The image processor is unavailable.";
					return;
				}
				originalContext.drawImage(bitmap, 0, 0);

				const thumbnailScale = Math.min(1, 600 / Math.max(bitmap.width, bitmap.height));
				const thumbnailWidth = Math.max(1, Math.round(bitmap.width * thumbnailScale));
				const thumbnailHeight = Math.max(1, Math.round(bitmap.height * thumbnailScale));
				const thumbnailCanvas = new OffscreenCanvas(thumbnailWidth, thumbnailHeight);
				const thumbnailContext = thumbnailCanvas.getContext("2d");
				if (thumbnailContext === null) {
					error = "The thumbnail processor is unavailable.";
					return;
				}
				thumbnailContext.drawImage(bitmap, 0, 0, thumbnailWidth, thumbnailHeight);

				const hashScale = Math.min(1, 100 / Math.max(bitmap.width, bitmap.height));
				const hashWidth = Math.max(1, Math.round(bitmap.width * hashScale));
				const hashHeight = Math.max(1, Math.round(bitmap.height * hashScale));
				const hashCanvas = new OffscreenCanvas(hashWidth, hashHeight);
				const hashContext = hashCanvas.getContext("2d");
				if (hashContext === null) {
					error = "The placeholder processor is unavailable.";
					return;
				}
				hashContext.drawImage(bitmap, 0, 0, hashWidth, hashHeight);
				const pixels = hashContext.getImageData(0, 0, hashWidth, hashHeight);
				const thumbHash = Array.from(rgbaToThumbHash(hashWidth, hashHeight, pixels.data), (byte) =>
					byte.toString(16).padStart(2, "0"),
				).join("");
				const width = bitmap.width;
				const height = bitmap.height;

				const original = await originalCanvas.convertToBlob({ type: "image/png" });
				const thumbnail = await thumbnailCanvas.convertToBlob({ type: "image/jpeg", quality: 0.9 });
				const input: PhotoUpload = {
					title: title.trim(),
					description: description.trim() === "" ? null : description.trim(),
					tags,
					date: date === "" ? null : new Date(date).toISOString(),
					geo: hasLatitude ? { lat, lng } : null,
					thumbHash,
					width,
					height,
				};
				const response = await invoke<CommandResponse<PhotoItem>>("create_photo", {
					input,
					original: Array.from(new Uint8Array(await original.arrayBuffer())),
					thumbnail: Array.from(new Uint8Array(await thumbnail.arrayBuffer())),
				});
				if (response.status === "failed") {
					error = response.message;
					return;
				}
				onuploaded(response.data);
				clearFile();
		} catch {
			error = "The photo could not be processed or uploaded.";
		} finally {
			bitmap?.close();
			uploading = false;
		}
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
		<label class="drop-zone">
			<Upload size={22} />
			<span>Choose an image</span>
			<small>PNG, JPEG, WebP or AVIF · maximum 20 MB</small>
			<input
				type="file"
				accept="image/png,image/jpeg,image/webp,image/avif"
				onchange={(event) => selectFile(event.currentTarget)}
			/>
		</label>
	{:else}
		<div class="upload-grid">
			<div class="preview">
				{#if preview !== null}<img src={preview} alt="Selected upload preview" />{/if}
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
	.drop-zone:hover { border-color: color-mix(in srgb, var(--color-accent) 50%, var(--color-border)); background: color-mix(in srgb, var(--color-accent) 5%, transparent); color: var(--color-foreground); }
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
