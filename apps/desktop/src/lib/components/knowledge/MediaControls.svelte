<script lang="ts">
	import { onMount } from "svelte";
	import { Maximize, Minimize, Pause, Play, Volume2, VolumeX } from "@lucide/svelte";

	let { media, frame }: { media: HTMLMediaElement; frame: HTMLElement } = $props();
	const video = $derived(media instanceof HTMLVideoElement);
	const title = $derived(media.getAttribute("aria-label") ?? (video ? "Video player" : "Audio player"));
	let paused = $state(true);
	let pending = $state(false);
	let waiting = $state(false);
	let current = $state(0);
	let duration = $state(0);
	let volume = $state(1);
	let muted = $state(false);
	let fullscreen = $state(false);
	let error = $state("");
	let active = false;
	let playRequest = 0;
	const showPlay = $derived(paused && !pending);
	const canSeek = $derived(Number.isFinite(duration) && duration > 0);

	function time(value: number) {
		if (!Number.isFinite(value) || value < 0) return "0:00";
		const seconds = Math.floor(value);
		return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
	}

	onMount(() => {
		active = true;
		function sync() {
			paused = media.paused;
			current = media.currentTime;
			duration = media.duration;
			volume = media.volume;
			muted = media.muted;
		}
		function loading() { waiting = true; }
		function ready() { waiting = false; }
		function failed() {
			waiting = false;
			error = "无法播放此媒体，请检查资源地址或格式。";
		}
		function fullscreenChanged() { fullscreen = document.fullscreenElement === frame; }
		const events = ["play", "pause", "ended", "timeupdate", "durationchange", "loadedmetadata", "volumechange"];
		for (const event of events) media.addEventListener(event, sync);
		for (const event of ["playing", "canplay", "pause", "ended"]) media.addEventListener(event, ready);
		media.addEventListener("waiting", loading);
		media.addEventListener("error", failed);
		document.addEventListener("fullscreenchange", fullscreenChanged);
		sync();
		if (media.error) failed();
		return () => {
			active = false;
			for (const event of events) media.removeEventListener(event, sync);
			for (const event of ["playing", "canplay", "pause", "ended"]) media.removeEventListener(event, ready);
			media.removeEventListener("waiting", loading);
			media.removeEventListener("error", failed);
			document.removeEventListener("fullscreenchange", fullscreenChanged);
		};
	});

	async function togglePlayback() {
		if (pending || !media.paused) {
			playRequest++;
			pending = false;
			waiting = false;
			media.pause();
			return;
		}
		const request = ++playRequest;
		error = "";
		pending = true;
		try {
			await media.play();
			if (!active) media.pause();
		} catch {
			if (active && request === playRequest) error = "无法开始播放，请检查资源地址或格式后重试。";
		} finally {
			if (active && request === playRequest) pending = false;
		}
	}

	function seek(event: Event) {
		if (!canSeek) return;
		media.currentTime = Number((event.currentTarget as HTMLInputElement).value);
		current = media.currentTime;
	}

	function changeVolume(event: Event) {
		media.volume = Number((event.currentTarget as HTMLInputElement).value);
		media.muted = false;
	}

	async function toggleFullscreen() {
		try {
			if (document.fullscreenElement === frame) await document.exitFullscreen();
			else await frame.requestFullscreen();
		} catch {
			if (active) error = "无法进入全屏，请重试。";
		}
	}
</script>

<div class="media-controls" role="group" aria-label={title}>
	{#if !video}<div class="media-title">{title}</div>{/if}
	<div class="transport">
		<button class="play" type="button" aria-label={showPlay ? "播放" : "暂停"} title={showPlay ? "播放" : "暂停"} onclick={() => void togglePlayback()}>
			{#if showPlay}<Play size={16} strokeWidth={1.6} fill="currentColor" aria-hidden="true" />{:else}<Pause size={16} strokeWidth={1.6} fill="currentColor" aria-hidden="true" />{/if}
		</button>
		<span class="time">{time(current)}</span>
		<input class="seek" type="range" aria-label="播放进度" aria-valuetext={`${time(current)} / ${canSeek ? time(duration) : "时长未知"}`} min="0" max={canSeek ? duration : 1} step="0.1" value={current} disabled={!canSeek} oninput={seek} style={`--played: ${canSeek ? Math.min(100, current / duration * 100) : 0}%`} />
		<span class="time">{canSeek ? time(duration) : "–:––"}</span>
		<button type="button" aria-label={muted || volume === 0 ? "取消静音" : "静音"} title={muted || volume === 0 ? "取消静音" : "静音"} onclick={() => { if (volume === 0) media.volume = 1; media.muted = !muted && volume !== 0; }}>
			{#if muted || volume === 0}<VolumeX size={17} strokeWidth={1.6} aria-hidden="true" />{:else}<Volume2 size={17} strokeWidth={1.6} aria-hidden="true" />{/if}
		</button>
		<input class="volume" type="range" aria-label="音量" min="0" max="1" step="0.05" value={muted ? 0 : volume} oninput={changeVolume} style={`--played: ${muted ? 0 : volume * 100}%`} />
		{#if video && document.fullscreenEnabled}
			<button type="button" aria-label={fullscreen ? "退出全屏" : "全屏"} title={fullscreen ? "退出全屏" : "全屏"} onclick={() => void toggleFullscreen()}>
				{#if fullscreen}<Minimize size={16} strokeWidth={1.6} aria-hidden="true" />{:else}<Maximize size={16} strokeWidth={1.6} aria-hidden="true" />{/if}
			</button>
		{/if}
	</div>
	{#if error}<div class="message error" role="alert">{error}</div>{:else if pending || waiting}<div class="message" role="status">正在加载…</div>{/if}
</div>

<style>
	.media-controls { container-type: inline-size; padding: 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.75rem; line-height: 1.4; letter-spacing: normal; }
	.media-title { overflow: hidden; margin: 0 0 0.5rem; color: var(--color-muted-foreground); font-size: 0.75rem; font-weight: 500; text-overflow: ellipsis; white-space: nowrap; }
	.transport { display: flex; align-items: center; gap: 0.625rem; min-width: 0; }
	button { display: inline-flex; flex: 0 0 auto; align-items: center; justify-content: center; width: 2rem; height: 2rem; padding: 0; border: 0; border-radius: var(--radius-md); background: transparent; color: var(--color-muted-foreground); cursor: pointer; transition: background var(--duration-fast), color var(--duration-fast); }
	button:hover { background: var(--color-muted); color: var(--color-foreground); }
	button.play { background: var(--color-accent); color: var(--color-accent-foreground); }
	button.play:hover { background: color-mix(in srgb, var(--color-accent) 88%, var(--color-foreground)); }
	button:disabled { opacity: 0.5; cursor: wait; }
	button:focus-visible, input:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 3px; }
	.time { flex: 0 0 auto; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.75rem; font-variant-numeric: tabular-nums; }
	input { appearance: none; height: 1.5rem; min-width: 0; margin: 0; padding: 0; border: 0; border-radius: var(--radius-full); background: transparent; cursor: pointer; }
	.seek { flex: 1 1 auto; width: 100%; }
	.volume { flex: 0 0 3.5rem; width: 3.5rem; }
	input::-webkit-slider-runnable-track { height: 3px; border-radius: var(--radius-full); background: linear-gradient(to right, var(--color-accent) var(--played), var(--color-border-strong) var(--played)); }
	input::-moz-range-track { height: 3px; border-radius: var(--radius-full); background: linear-gradient(to right, var(--color-accent) var(--played), var(--color-border-strong) var(--played)); }
	input::-webkit-slider-thumb { appearance: none; width: 10px; height: 10px; margin-top: -3.5px; border: 0; border-radius: var(--radius-full); background: var(--color-accent); }
	input::-moz-range-thumb { width: 10px; height: 10px; border: 0; border-radius: var(--radius-full); background: var(--color-accent); }
	input:disabled { opacity: 0.4; cursor: default; }
	.message { margin-top: 0.5rem; color: var(--color-muted-foreground); }
	.error { color: var(--color-error); }
	:global(.content-embed-media:fullscreen) { display: flex; flex-direction: column; justify-content: center; width: 100%; height: 100%; padding: 1rem; box-sizing: border-box; background: var(--color-background); }
	:global(.content-embed-media:fullscreen > video) { min-height: 0; max-height: calc(100% - 6rem); }
	@container (max-width: 340px) { .volume { display: none; } .transport { gap: 0.375rem; } }
</style>
