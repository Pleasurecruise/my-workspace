<script module lang="ts">
	import type { MusicProvider, MusicTrack } from "../consumer";

	let settledProvider: MusicProvider = "qqMusic";
	let settledTracks: Record<MusicProvider, MusicTrack[] | null> = {
		spotify: null,
		qqMusic: null,
	};
</script>

<script lang="ts">
	import { convertFileSrc, invoke } from "@tauri-apps/api/core";
	import { ListOrdered, LoaderCircle, Music2, Pause, Play, Repeat1, Shuffle, SkipBack, SkipForward } from "@lucide/svelte";
	import { onMount } from "svelte";
	import type { CommandResponse, MusicLyrics, MusicPlayback } from "../consumer";

	let {
		onopensettings,
		onopenplayer,
		playerVisible = $bindable(false),
		playerAvailable = $bindable(false),
	}: { onopensettings: () => void; onopenplayer: () => void; playerVisible?: boolean; playerAvailable?: boolean } = $props();

	const initialTracks = settledTracks[settledProvider];
	let provider = $state<MusicProvider>(settledProvider);
	let tracks = $state<MusicTrack[]>(initialTracks === null ? [] : initialTracks);
	let selectedTrack = $state<MusicTrack | null>(null);
	let playback = $state<MusicPlayback | null>(null);
	let lyrics = $state<MusicLyrics | null>(null);
	let loading = $state(settledTracks[settledProvider] === null);
	let acting = $state(false);
	let lyricsLoading = $state(false);
	let error = $state<string | null>(null);
	let lyricsError = $state<string | null>(null);
	let progressMs = $state(0);
	let tracksRequest = 0;
	let playbackRequest = 0;
	let lyricsRequest = 0;
	let playbackError = $state<string | null>(null);
	let providerName = $derived(provider === "spotify" ? "Spotify" : "QQ Music");
	let collectionName = $derived(provider === "spotify" ? "Liked Songs" : "Daily 30");
	let selectedIndex = $derived(selectedTrack === null ? -1 : tracks.findIndex((track) => track.id === selectedTrack?.id));
	let playbackOrder = $derived(playback === null ? "sequential" : playback.order);
	let playbackOrderLabel = $derived(playbackOrder === "sequential" ? "Sequential" : playbackOrder === "repeatOne" ? "Repeat one" : "Shuffle");
	let activeLine = $derived.by(() => {
		if (lyrics === null || !lyrics.synced) return -1;
		let active = -1;
		for (let index = 0; index < lyrics.lines.length; index += 1) {
			const line = lyrics.lines[index]!;
			const start = line.startMs;
			if (start === null || start > progressMs) break;
			active = index;
		}
		return active;
	});
	let subtitleText = $derived.by(() => {
		if (lyricsLoading) return "Finding lyrics…";
		if (lyricsError !== null) return lyricsError;
		if (lyrics?.instrumental) return "Instrumental";
		if (lyrics === null || lyrics.lines.length === 0) return "Lyrics unavailable";
		if (lyrics.synced) return activeLine < 0 ? "…" : lyrics.lines[activeLine]!.text;
		const duration = selectedTrack === null ? 0 : selectedTrack.durationMs;
		const ratio = duration > 0 ? Math.min(1, progressMs / duration) : 0;
		const index = Math.min(lyrics.lines.length - 1, Math.floor(ratio * lyrics.lines.length));
		return lyrics.lines[index]!.text;
	});

	function coverSource(track: MusicTrack): string | null {
		return track.coverKey === null ? null : convertFileSrc(track.coverKey, "vesper-music-cover");
	}

	async function loadTracks(target = provider) {
		const version = ++tracksRequest;
		loading = tracks.length === 0;
		error = null;
		const response = await invoke<CommandResponse<MusicTrack[]>>("read_music_tracks", { provider: target });
		if (provider !== target || version !== tracksRequest) return;
		loading = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		tracks = response.data;
		settledTracks[target] = response.data;
		await syncPlayback();
	}

	function selectProvider(next: MusicProvider) {
		if (provider === next || acting) return;
		tracksRequest += 1;
		playbackRequest += 1;
		lyricsRequest += 1;
		playbackError = null;
		provider = next;
		settledProvider = next;
		const cachedTracks = settledTracks[next];
		tracks = cachedTracks === null ? [] : cachedTracks;
		selectedTrack = null;
		playerVisible = false;
		playerAvailable = false;
		playback = null;
		lyrics = null;
		lyricsLoading = false;
		progressMs = 0;
		void loadTracks(next);
	}

	async function syncPlayback() {
		if (acting) return;
		const version = ++playbackRequest;
		const target = provider;
		const response = await invoke<CommandResponse<MusicPlayback | null>>("read_music_playback", { provider: target });
		if (provider !== target || version !== playbackRequest) return;
		if (response.status === "failed") {
			playbackError = response.message;
			return;
		}
		playbackError = null;
		playback = response.data;
		progressMs = response.data === null ? 0 : response.data.progressMs;
		const playingTrack = tracks.find((track) => track.id === response.data?.trackId);
		playerAvailable = playingTrack !== undefined;
		if (playingTrack && playingTrack.id !== selectedTrack?.id) {
			await selectTrack(playingTrack, false, playerVisible);
		}
	}

	async function selectTrack(track: MusicTrack, startPlayback = true, showPlayer = true) {
		if (acting) return;
		const target = provider;
		if (startPlayback) {
			const version = ++playbackRequest;
			acting = true;
			error = null;
			const response = await invoke<CommandResponse<string>>("play_music_track", { provider: target, trackId: track.id });
			if (provider !== target || version !== playbackRequest) return;
			acting = false;
			if (response.status === "failed") {
				error = response.message;
				return;
			}
			playback = { trackId: track.id, playing: true, progressMs: 0, durationMs: track.durationMs, order: playbackOrder };
			playerAvailable = true;
		}
		const version = ++lyricsRequest;
		selectedTrack = track;
		if (showPlayer && !playerVisible) onopenplayer();
		playerVisible = showPlayer;
		lyrics = null;
		lyricsError = null;
		progressMs = playback === null ? 0 : playback.progressMs;
		lyricsLoading = true;
		const response = await invoke<CommandResponse<MusicLyrics | null>>("read_music_lyrics", { provider: target, trackId: track.id });
		if (provider !== target || version !== lyricsRequest) return;
		lyricsLoading = false;
		if (response.status === "ready") lyrics = response.data;
		else lyricsError = response.message;
	}

	async function togglePlayback() {
		if (selectedTrack === null || acting) return;
		const version = ++playbackRequest;
		acting = true;
		const playing = playback?.playing === true;
		const response = playback === null
			? await invoke<CommandResponse<string>>("play_music_track", { provider, trackId: selectedTrack.id })
			: await invoke<CommandResponse<string>>(playing ? "pause_music" : "resume_music", { provider });
		if (version !== playbackRequest) return;
		acting = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		playback = {
			trackId: selectedTrack.id,
			playing: playback === null || !playing,
			progressMs: playback === null ? 0 : progressMs,
			durationMs: selectedTrack.durationMs,
			order: playbackOrder,
		};
	}

	async function cyclePlaybackOrder() {
		if (acting) return;
		const version = ++playbackRequest;
		const order: MusicPlayback["order"] = playbackOrder === "sequential" ? "repeatOne" : playbackOrder === "repeatOne" ? "shuffle" : "sequential";
		acting = true;
		const response = await invoke<CommandResponse<string>>("set_music_playback_order", { provider, order });
		if (version !== playbackRequest) return;
		acting = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		if (playback !== null) playback = { ...playback, order };
	}

	async function move(offset: number) {
		const next = tracks[selectedIndex + offset];
		if (next) await selectTrack(next);
	}

	async function seek(position: number) {
		if (acting || selectedTrack === null) return;
		const version = ++playbackRequest;
		acting = true;
		progressMs = position;
		const response = await invoke<CommandResponse<string>>("seek_music", { provider, positionMs: Math.round(position) });
		if (version !== playbackRequest) return;
		acting = false;
		if (response.status === "failed") error = response.message;
	}

	function formatTime(milliseconds: number) {
		const seconds = Math.max(0, Math.floor(milliseconds / 1_000));
		return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
	}

	onMount(() => {
		void loadTracks();
		const progressTimer = window.setInterval(() => {
			if (playback?.playing !== true || selectedTrack === null) return;
			progressMs = Math.min(selectedTrack.durationMs, progressMs + 500);
		}, 500);
		const syncTimer = window.setInterval(() => void syncPlayback(), 5_000);
		return () => {
			tracksRequest += 1;
			playbackRequest += 1;
			lyricsRequest += 1;
			window.clearInterval(progressTimer);
			window.clearInterval(syncTimer);
		};
	});
</script>

<section class="music" class:player-visible={playerVisible} aria-label={`${providerName} ${collectionName}`}>
	{#if !playerVisible}
	<header>
		<div><p>{providerName}</p><h1>{collectionName}</h1><span>{tracks.length} songs</span></div>
		<div class="provider-switch" aria-label="Music source">
			<button type="button" class:active={provider === "qqMusic"} aria-pressed={provider === "qqMusic"} disabled={acting} onclick={() => selectProvider("qqMusic")}>QQ Music</button>
			<button type="button" class:active={provider === "spotify"} aria-pressed={provider === "spotify"} disabled={acting} onclick={() => selectProvider("spotify")}>Spotify</button>
		</div>
	</header>
	{/if}

	{#if error !== null}
		<div class="notice" role="alert"><span>{error}</span>{#if tracks.length === 0}<button type="button" onclick={onopensettings}>Open Settings</button>{/if}</div>
	{/if}

	{#if playbackError !== null}<div class="notice" role="alert">{playbackError}</div>{/if}

	{#if loading}
		<div class="loading"><LoaderCircle size={18} /> Loading {collectionName}…</div>
	{:else if tracks.length === 0 && error === null}
		<div class="empty"><HeartMark /><h2>No songs available</h2><p>{provider === "spotify" ? "Songs you save in Spotify will appear here." : "QQ Music did not return today's recommendations."}</p></div>
	{:else if playerVisible && selectedTrack !== null}
		<div class="player-page" aria-label="Music player">
			<div class="record-column">
				<div class="record-wrap">
					<div class="record" class:spinning={playback?.playing === true}>
						{#if coverSource(selectedTrack) !== null}<img src={coverSource(selectedTrack)} alt={`${selectedTrack.album} cover`} />{:else}<Music2 size={44} />{/if}
						<span></span>
					</div>
				</div>
				<div class="subtitle" aria-live="polite">
					{#key subtitleText}<p>{subtitleText}</p>{/key}
				</div>
				<div class="now-playing"><strong>{selectedTrack.name}</strong><span>{selectedTrack.artists.join(", ")}</span></div>
				<div class="transport">
					<button type="button" class:active={playbackOrder !== "sequential"} disabled={acting} onclick={cyclePlaybackOrder} aria-label={`Playback order: ${playbackOrderLabel}`} title={playbackOrderLabel}>
						{#if playbackOrder === "repeatOne"}<Repeat1 size={19} />{:else if playbackOrder === "shuffle"}<Shuffle size={19} />{:else}<ListOrdered size={19} />{/if}
					</button>
					<button type="button" disabled={selectedIndex <= 0 || acting} onclick={() => move(-1)} aria-label="Previous song"><SkipBack size={20} fill="currentColor" /></button>
					<button type="button" class="play" disabled={acting} onclick={togglePlayback} aria-label={playback?.playing ? "Pause" : "Play"}>{#if playback?.playing}<Pause size={22} fill="currentColor" />{:else}<Play size={22} fill="currentColor" />{/if}</button>
					<button type="button" disabled={selectedIndex < 0 || selectedIndex >= tracks.length - 1 || acting} onclick={() => move(1)} aria-label="Next song"><SkipForward size={20} fill="currentColor" /></button>
					<span class="transport-spacer" aria-hidden="true"></span>
				</div>
				<div class="timeline"><span>{formatTime(progressMs)}</span><input type="range" disabled={acting} min="0" max={selectedTrack.durationMs} value={progressMs} oninput={(event) => (progressMs = event.currentTarget.valueAsNumber)} onchange={(event) => seek(event.currentTarget.valueAsNumber)} aria-label="Playback position" /><span>{formatTime(selectedTrack.durationMs)}</span></div>
			</div>
		</div>
	{:else}
		<div class="track-list" aria-label={collectionName}>
			{#each tracks as track, index (track.id)}
				<button type="button" disabled={acting} onclick={() => selectTrack(track)}>
					<span class="track-number">{index + 1}</span>
					<span class="cover">
						{#if coverSource(track) !== null}<img src={coverSource(track)} alt="" loading="lazy" />{:else}<Music2 size={16} />{/if}
						<span class="play-overlay"><Play size={14} fill="currentColor" /></span>
					</span>
					<span class="track-title"><strong>{track.name}</strong><small>{track.artists.join(", ")}</small></span>
					<span class="album">{track.album}</span>
					<time>{formatTime(track.durationMs)}</time>
				</button>
			{/each}
		</div>
	{/if}
</section>

{#snippet HeartMark()}<div class="heart-mark" aria-hidden="true">♥</div>{/snippet}

<style>
	.music { width: min(100%, 76rem); margin: 0 auto; }
	.music.player-visible { display: flex; flex: 1; flex-direction: column; }
	header { display: flex; align-items: end; justify-content: space-between; gap: 1rem; margin-bottom: 1.25rem; }
	header p, header h1, header span { margin: 0; }
	header p { margin-bottom: 0.3rem; color: var(--color-accent); font-size: 0.68rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	header h1 { font-family: var(--font-serif); font-size: 2rem; font-weight: 500; }
	header span { color: var(--color-muted-foreground); font-size: 0.7rem; }
	.provider-switch { display: inline-flex; padding: 0.2rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-muted); }
	.provider-switch button { padding: 0.38rem 0.65rem; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font: inherit; font-size: 0.68rem; }
	.provider-switch button.active { background: var(--color-card); color: var(--color-foreground); box-shadow: var(--shadow-sm); }
	.provider-switch button:disabled { cursor: default; opacity: 0.55; }
	.provider-switch button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 1px; }
	.notice { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1rem; padding: 0.7rem 0.85rem; border: 1px solid var(--color-error); border-radius: var(--radius-md); color: var(--color-error); font-size: 0.72rem; }
	.notice button { border: 0; background: transparent; color: inherit; cursor: pointer; font: inherit; text-decoration: underline; }
	.loading, .empty { display: grid; min-height: 20rem; place-items: center; align-content: center; gap: 0.65rem; color: var(--color-muted-foreground); text-align: center; }
	.loading :global(svg) { animation: spin 1s linear infinite; }
	.empty h2, .empty p { margin: 0; }
	.empty h2 { color: var(--color-foreground); font-family: var(--font-serif); font-weight: 500; }
	.empty p { max-width: 18rem; font-size: 0.72rem; }
	.heart-mark { display: grid; width: 4rem; height: 4rem; place-items: center; border-radius: var(--radius-lg); background: var(--color-accent); color: var(--color-accent-foreground); font-size: 2rem; }
	.track-list { overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-card); }
	.track-list > button { display: grid; width: 100%; grid-template-columns: 1.5rem 2.6rem minmax(9rem, 1.2fr) minmax(8rem, 0.8fr) 2.5rem; align-items: center; gap: 0.65rem; padding: 0.5rem 0.7rem; border: 0; border-bottom: 1px solid var(--color-border); background: transparent; color: var(--color-foreground); cursor: pointer; text-align: left; }
	.track-list > button:last-child { border-bottom: 0; }
	.track-list > button:not(:disabled):hover { background: var(--color-muted); }
	.track-list > button:disabled { cursor: default; opacity: 0.65; }
	.track-number, time, .album, .track-title small { color: var(--color-muted-foreground); font-size: 0.65rem; }
	.cover { position: relative; display: grid; width: 2.6rem; aspect-ratio: 1; place-items: center; overflow: hidden; border-radius: var(--radius-sm); background: var(--color-muted); }
	.cover img { width: 100%; height: 100%; object-fit: cover; }
	.play-overlay { position: absolute; inset: 0; display: grid; place-items: center; background: var(--color-image-scrim); color: var(--color-on-dark); opacity: 0; }
	.track-list > button:not(:disabled):hover .play-overlay { opacity: 1; }
	.track-title { display: grid; min-width: 0; gap: 0.18rem; }
	.track-title strong, .track-title small, .album { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.track-title strong { font-size: 0.72rem; font-weight: 600; }
	.player-page { display: grid; flex: 1; width: min(100%, 50rem); min-height: 34rem; grid-template-columns: minmax(0, 1fr); gap: 1rem; margin: 0 auto; }
	.record-column { display: flex; min-width: 0; align-items: center; flex-direction: column; justify-content: center; padding: 2rem 1.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-card); box-shadow: var(--shadow-sm); }
	.record-wrap { display: grid; width: 100%; min-height: 19rem; flex: 1; place-items: center; }
	.record { position: relative; display: grid; width: min(18rem, 100%); box-sizing: border-box; aspect-ratio: 1; place-items: center; overflow: hidden; border: 1rem solid var(--color-image-scrim); border-radius: 50%; background: repeating-radial-gradient(circle, var(--color-image-scrim) 0 0.35rem, var(--color-background) 0.38rem, var(--color-image-scrim) 0.42rem); box-shadow: var(--shadow-lg); }
	.record::before { position: absolute; inset: 8%; border: 1px solid var(--color-border); border-radius: 50%; content: ""; box-shadow: 0 0 0 0.7rem var(--color-image-scrim), 0 0 0 0.75rem var(--color-border), 0 0 0 1.4rem var(--color-image-scrim); }
	.record img { z-index: 1; width: 58%; aspect-ratio: 1; border-radius: 50%; object-fit: cover; }
	.record > span { position: absolute; z-index: 2; width: 0.7rem; aspect-ratio: 1; border-radius: 50%; background: var(--color-card); }
	.record.spinning { animation: record-spin 12s linear infinite; }
	.subtitle { display: grid; width: min(100%, 38rem); min-height: 3.8rem; place-items: center; overflow: hidden; padding: 0.5rem 1rem; box-sizing: border-box; text-align: center; }
	.subtitle p { margin: 0; color: var(--color-foreground); font-family: var(--font-serif); font-size: 1.05rem; font-weight: 600; line-height: 1.55; animation: subtitle-in var(--duration-base) ease-out; }
	.now-playing { display: grid; width: 100%; gap: 0.22rem; margin: 1rem 0 0.9rem; text-align: center; }
	.now-playing strong { font-family: var(--font-serif); font-size: 1.3rem; font-weight: 600; }
	.now-playing span { color: var(--color-muted-foreground); font-size: 0.68rem; }
	.transport { display: flex; align-items: center; justify-content: center; gap: 0.95rem; margin-top: 0.8rem; }
	.transport button { display: grid; width: 2rem; aspect-ratio: 1; place-items: center; border: 0; border-radius: 50%; background: transparent; color: var(--color-foreground); cursor: pointer; }
	.transport button.play { width: 2.8rem; background: var(--color-foreground); color: var(--color-background); }
	.transport button.active { color: var(--color-accent); }
	.transport button:disabled { cursor: default; opacity: 0.35; }
	.transport-spacer { width: 2rem; }
	.timeline { display: grid; width: min(100%, 36rem); grid-template-columns: 2.2rem 1fr 2.2rem; align-items: center; gap: 0.4rem; margin-top: 0.85rem; color: var(--color-muted-foreground); font-size: 0.58rem; }
	.timeline span:last-child { text-align: right; }
	.timeline input { width: 100%; accent-color: var(--color-accent); }
	@keyframes record-spin { to { transform: rotate(360deg); } }
	@keyframes subtitle-in { from { opacity: 0; transform: translateY(0.6rem); } to { opacity: 1; transform: translateY(0); } }
	@keyframes spin { to { transform: rotate(360deg); } }
	@media (max-width: 640px) { header { align-items: stretch; flex-direction: column; } .record-column, .track-list { width: 100%; box-sizing: border-box; } .album { display: none; } .track-list > button { grid-template-columns: 1.2rem 2.6rem minmax(0, 1fr) 2.5rem; } }
	@media (prefers-reduced-motion: reduce) { .record.spinning, .loading :global(svg), .subtitle p { animation: none; } }
</style>
