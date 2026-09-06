<script lang="ts">
	import { convertFileSrc } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import { thumbHashToDataURL } from "thumbhash";

	let {
		objectKey,
		previewObjectKey = null,
		thumbHash,
		alt,
		width,
		height,
		eager = false,
		fit = "cover",
		retryable = false,
	}: {
		objectKey: string;
		previewObjectKey?: string | null;
		thumbHash: string | null;
		alt: string;
		width: number;
		height: number;
		eager?: boolean;
		fit?: "cover" | "contain";
		retryable?: boolean;
	} = $props();

	let container = $state<HTMLDivElement | null>(null);
	let originalReady = $state(false);
	let originalFailed = $state(false);
	let previewFailed = $state(false);
	let visible = $state(false);
	let requestRevision = $state(0);
	let thumbHashSource = $derived(decodeThumbHash(thumbHash));
	let previewSource = $derived(previewObjectKey === null ? null : convertFileSrc(previewObjectKey, "vesper-asset"));
	let originalSource = $derived(`${convertFileSrc(objectKey, "vesper-asset")}?revision=${requestRevision}`);

	function decodeThumbHash(hash: string | null): string | null {
		if (hash === null || hash.length === 0 || hash.length % 2 !== 0 || !/^[\da-f]+$/i.test(hash)) {
			return null;
		}
		const bytes = hash.match(/.{2}/g);
		if (bytes === null) return null;
		const decoded = Uint8Array.from(bytes, (byte) => Number.parseInt(byte, 16));
		return thumbHashToDataURL(decoded);
	}

	function revealOriginal(event: Event) {
		if (!(event.currentTarget instanceof HTMLImageElement)) return;
		const image = event.currentTarget;
		const source = image.currentSrc;
		void image.decode().then(
			() => {
				if (image.isConnected && image.currentSrc === source) originalReady = true;
			},
			() => {
				if (image.isConnected && image.currentSrc === source) originalFailed = true;
			},
		);
	}

	function retry() {
		originalReady = false;
		originalFailed = false;
		requestRevision += 1;
	}

	onMount(() => {
		if (eager) {
			visible = true;
			return;
		}
		const observer = new IntersectionObserver(
			(entries) => {
				if (!entries.some((entry) => entry.isIntersecting)) return;
				visible = true;
				observer.disconnect();
			},
			{ rootMargin: "400px" },
		);
		if (container !== null) observer.observe(container);

		return () => {
			observer.disconnect();
		};
	});
</script>

<div
	bind:this={container}
	class="progressive-image"
	class:contain={fit === "contain"}
	class:unavailable={originalFailed && previewSource === null && thumbHashSource === null}
	style={`aspect-ratio: ${width > 0 && height > 0 ? `${width} / ${height}` : "4 / 3"}`}
>
	{#if thumbHashSource !== null}
		<img
			class="thumbhash"
			class:hidden={originalReady}
			src={thumbHashSource}
			alt=""
			aria-hidden="true"
		/>
	{/if}
	{#if previewSource !== null && !previewFailed}
		<img
			class="preview"
			class:hidden={originalReady}
			src={previewSource}
			alt=""
			aria-hidden="true"
			onerror={() => (previewFailed = true)}
		/>
	{/if}
	{#if visible}
		<img
			class="original"
			class:ready={originalReady}
			src={originalSource}
			{alt}
			{width}
			{height}
			decoding="async"
			onload={revealOriginal}
			onerror={() => (originalFailed = true)}
		/>
	{/if}
	{#if originalFailed}
		<div class="image-error" role="alert">
			<span>Full-resolution image unavailable</span>
			{#if retryable}<button type="button" onclick={retry}>Retry</button>{/if}
		</div>
	{/if}
</div>

<style>
	.progressive-image { position: relative; display: block; width: 100%; overflow: hidden; }
	img { position: absolute; inset: 0; display: block; width: 100%; height: 100%; object-fit: cover; }
	.contain img { object-fit: contain; }
	.thumbhash { scale: 1.1; filter: blur(0.25rem); }
	.preview.hidden, .thumbhash.hidden { opacity: 0; }
	.original { opacity: 0; }
	.original.ready { opacity: 1; }
	.unavailable { display: grid; min-height: 12rem; place-items: center; color: var(--color-error); font-size: 0.68rem; }
	.image-error { position: absolute; inset: auto 0 0; z-index: 1; display: flex; align-items: center; justify-content: center; gap: 0.6rem; padding: 0.55rem; background: var(--color-image-scrim); color: var(--color-on-dark); font-size: 0.68rem; }
	.image-error button { padding: 0.2rem 0.5rem; border: 1px solid currentColor; border-radius: var(--radius-sm); background: transparent; color: inherit; cursor: pointer; font: inherit; }
</style>
