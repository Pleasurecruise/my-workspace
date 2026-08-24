<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import { thumbHashToDataURL } from "thumbhash";
	import type { CommandResponse } from "../consumer";

	let {
		objectKey,
		thumbHash,
		alt,
		width,
		height,
	}: {
		objectKey: string;
		thumbHash: string | null;
		alt: string;
		width: number;
		height: number;
	} = $props();

	let container = $state<HTMLDivElement | null>(null);
	let originalSource = $state<string | null>(null);
	let originalReady = $state(false);
	let originalFailed = $state(false);
	let visible = $state(false);
	let originalRequested = false;
	let active = false;
	let thumbHashSource = $derived(decodeThumbHash(thumbHash));

	function decodeThumbHash(hash: string | null): string | null {
		if (hash === null || hash.length === 0 || hash.length % 2 !== 0 || !/^[\da-f]+$/i.test(hash)) {
			return null;
		}
		const bytes = hash.match(/.{2}/g);
		if (bytes === null) return null;
		const decoded = Uint8Array.from(bytes, (byte) => Number.parseInt(byte, 16));
		return thumbHashToDataURL(decoded);
	}

	async function loadOriginal() {
		if (originalRequested || !visible) return;
		originalRequested = true;
		const response = await invoke<CommandResponse<number[]>>("read_asset", {
			key: objectKey,
		});
		if (!active) return;
		if (response.status === "failed") {
			originalFailed = true;
			return;
		}
		const separator = objectKey.lastIndexOf(".");
		if (separator === -1) {
			originalFailed = true;
			return;
		}
		let contentType: string;
		switch (objectKey.slice(separator + 1).toLowerCase()) {
			case "png":
				contentType = "image/png";
				break;
			case "webp":
				contentType = "image/webp";
				break;
			case "avif":
				contentType = "image/avif";
				break;
			case "jpg":
			case "jpeg":
				contentType = "image/jpeg";
				break;
			default:
				originalFailed = true;
				return;
		}
		originalSource = URL.createObjectURL(
			new Blob([new Uint8Array(response.data)], { type: contentType }),
		);
	}

	onMount(() => {
		active = true;
		const observer = new IntersectionObserver(
			(entries) => {
				if (!entries.some((entry) => entry.isIntersecting)) return;
				visible = true;
				void loadOriginal();
				observer.disconnect();
			},
			{ rootMargin: "400px" },
		);
		if (container !== null) observer.observe(container);

		return () => {
			active = false;
			observer.disconnect();
			if (originalSource !== null) URL.revokeObjectURL(originalSource);
		};
	});
</script>

<div
	bind:this={container}
	class="progressive-image"
	class:unavailable={originalFailed && thumbHashSource === null}
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
	{#if originalSource !== null}
		<img
			class="original"
			class:ready={originalReady}
			src={originalSource}
			{alt}
			{width}
			{height}
			decoding="async"
			onload={() => (originalReady = true)}
			onerror={() => (originalFailed = true)}
		/>
	{/if}
	{#if originalFailed && thumbHashSource === null}
		<span role="img" aria-label={`${alt}: image unavailable`}>Image unavailable</span>
	{/if}
</div>

<style>
	.progressive-image { position: relative; display: block; width: 100%; overflow: hidden; }
	img { position: absolute; inset: 0; display: block; width: 100%; height: 100%; object-fit: cover; }
	.thumbhash { scale: 1.1; filter: blur(0.25rem); transition: opacity var(--duration-slow); }
	.thumbhash.hidden { opacity: 0; }
	.original { opacity: 0; transition: opacity var(--duration-slow); }
	.original.ready { opacity: 1; }
	.unavailable { display: grid; min-height: 12rem; place-items: center; color: var(--color-error); font-size: 0.68rem; }
	@media (prefers-reduced-motion: reduce) { .thumbhash, .original { transition: none; } }
</style>
