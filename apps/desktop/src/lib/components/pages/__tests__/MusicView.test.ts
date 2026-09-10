import { expect, it, vi } from "vite-plus/test";
import { mount, unmount } from "svelte";
import type { CommandResponse, MusicLyrics, MusicPlayback, MusicTrack } from "../../../consumer";
import MusicView from "../MusicView.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: vi.fn() }));

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (!element) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("ignores stale playback and lyrics, and preserves the playing song when switching fails", async () => {
	vi.useFakeTimers();
	const tracks: MusicTrack[] = [
		{
			id: "a",
			name: "Track A",
			artists: ["Artist"],
			album: "Album",
			durationMs: 100000,
			addedAt: "2026-09-05",
			coverKey: null,
		},
		{
			id: "b",
			name: "Track B",
			artists: ["Artist"],
			album: "Album",
			durationMs: 100000,
			addedAt: "2026-09-05",
			coverKey: null,
		},
	];
	const initialPlayback: CommandResponse<MusicPlayback> = {
		status: "ready",
		data: { trackId: "a", playing: true, progressMs: 0, durationMs: 100000, order: "sequential" },
	};
	let resolvePoll: (response: CommandResponse<MusicPlayback>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const poll = new Promise<CommandResponse<MusicPlayback>>((resolve) => {
		resolvePoll = resolve;
	});
	let resolvePlay: (response: CommandResponse<string>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const play = new Promise<CommandResponse<string>>((resolve) => {
		resolvePlay = resolve;
	});
	let resolveOldLyrics: (response: CommandResponse<MusicLyrics>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const oldLyrics = new Promise<CommandResponse<MusicLyrics>>((resolve) => {
		resolveOldLyrics = resolve;
	});
	let resolveNewLyrics: (response: CommandResponse<MusicLyrics>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const newLyrics = new Promise<CommandResponse<MusicLyrics>>((resolve) => {
		resolveNewLyrics = resolve;
	});
	let playbackReads = 0;
	let lyricReads = 0;
	let playRequests = 0;
	invoke.mockImplementation((command: string) => {
		if (command === "read_music_tracks") return Promise.resolve({ status: "ready", data: tracks });
		if (command === "read_music_playback")
			return ++playbackReads === 1 ? Promise.resolve(initialPlayback) : poll;
		if (command === "read_music_lyrics") return ++lyricReads === 1 ? oldLyrics : newLyrics;
		if (command === "play_music_track")
			return ++playRequests === 1
				? play
				: Promise.resolve({ status: "failed", message: "Playback rejected" });
		throw new Error(`Unexpected command: ${command}`);
	});
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(MusicView, {
		target,
		props: { playerVisible: true, onopenplayer: vi.fn(), onopensettings: vi.fn() },
	});
	await vi.waitFor(() =>
		expect(target.querySelector(".now-playing strong")?.textContent).toBe("Track A"),
	);
	await vi.advanceTimersByTimeAsync(5000);
	findElement<HTMLButtonElement>(target, '[aria-label="Next song"]').click();
	await vi.waitFor(() => expect(playRequests).toBe(1));
	resolvePoll(initialPlayback);
	resolvePlay({ status: "ready", data: "playing" });
	await vi.waitFor(() =>
		expect(target.querySelector(".now-playing strong")?.textContent).toBe("Track B"),
	);
	resolveOldLyrics({
		status: "ready",
		data: { synced: false, instrumental: false, lines: [{ startMs: null, text: "Old lyrics" }] },
	});
	await vi.advanceTimersByTimeAsync(1);
	expect(target.querySelector(".subtitle")?.textContent).toContain("Finding lyrics");
	resolveNewLyrics({
		status: "ready",
		data: { synced: false, instrumental: false, lines: [{ startMs: null, text: "New lyrics" }] },
	});
	await vi.waitFor(() =>
		expect(target.querySelector(".subtitle")?.textContent).toContain("New lyrics"),
	);
	findElement<HTMLButtonElement>(target, '[aria-label="Previous song"]').click();
	await vi.waitFor(() => expect(target.textContent).toContain("Playback rejected"));
	expect(target.querySelector(".now-playing strong")?.textContent).toBe("Track B");
	expect(lyricReads).toBe(2);
	const previous = findElement<HTMLButtonElement>(target, '[aria-label="Previous song"]');
	expect(previous.disabled).toBe(false);
	previous.click();
	await vi.waitFor(() => expect(playRequests).toBe(3));
	await vi.waitFor(() => expect(previous.disabled).toBe(false));
	await unmount(view);
	target.remove();
	vi.useRealTimers();
});

it("clears the previous Spotify collection after reconnecting and can retry a limited library", async () => {
	const tracks: MusicTrack[] = [
		{
			id: "old",
			name: "Previous account song",
			artists: ["Artist"],
			album: "Album",
			durationMs: 100000,
			addedAt: "2026-09-05",
			coverKey: null,
		},
	];
	let limited = false;
	invoke.mockImplementation((command: string) => {
		if (command === "read_music_tracks")
			return Promise.resolve(
				limited
					? { status: "failed", message: "Spotify returned 429. Retry in 60 seconds." }
					: { status: "ready", data: tracks },
			);
		if (command === "read_music_playback") return Promise.resolve({ status: "ready", data: null });
		throw new Error(`Unexpected command: ${command}`);
	});
	const target = document.createElement("div");
	document.body.append(target);
	const first = mount(MusicView, {
		target,
		props: { spotifyRevision: 100, onopenplayer: vi.fn(), onopensettings: vi.fn() },
	});
	await vi.waitFor(() => expect(target.textContent).toContain("Previous account song"));
	const spotify = Array.from(target.querySelectorAll(".provider-switch button")).find(
		(button) => button.textContent === "Spotify",
	);
	if (!(spotify instanceof HTMLButtonElement)) throw new Error("Missing Spotify button");
	spotify.click();
	await vi.waitFor(() => expect(spotify.getAttribute("aria-pressed")).toBe("true"));
	await vi.waitFor(() => expect(target.textContent).toContain("Previous account song"));
	await unmount(first);
	limited = true;
	const second = mount(MusicView, {
		target,
		props: { spotifyRevision: 101, onopenplayer: vi.fn(), onopensettings: vi.fn() },
	});
	await vi.waitFor(() => expect(target.textContent).toContain("Spotify returned 429"));
	expect(target.textContent).not.toContain("Previous account song");
	limited = false;
	const retry = Array.from(target.querySelectorAll("button")).find(
		(button) => button.textContent === "Retry library",
	);
	if (!(retry instanceof HTMLButtonElement)) throw new Error("Missing retry button");
	retry.click();
	await vi.waitFor(() => expect(target.textContent).toContain("Previous account song"));
	await unmount(second);
	target.remove();
});

it("rejects a previous account response that arrives after reconnection", async () => {
	const oldTrack: MusicTrack = {
		id: "old-account",
		name: "Old account response",
		artists: ["Artist"],
		album: "Album",
		durationMs: 100000,
		addedAt: "2026-09-05",
		coverKey: null,
	};
	const currentTrack = { ...oldTrack, id: "current-account", name: "Current account song" };
	const completions = new EventTarget();
	const oldRead = new Promise<CommandResponse<MusicTrack[]>>((resolve) => {
		completions.addEventListener("old", () => resolve({ status: "ready", data: [oldTrack] }), {
			once: true,
		});
	});
	invoke.mockImplementation((command: string) =>
		command === "read_music_tracks" ? oldRead : Promise.resolve({ status: "ready", data: null }),
	);
	const target = document.createElement("div");
	document.body.append(target);
	const first = mount(MusicView, {
		target,
		props: { spotifyRevision: 200, onopenplayer: vi.fn(), onopensettings: vi.fn() },
	});
	await vi.waitFor(() => expect(target.querySelector(".provider-switch")).not.toBeNull());
	const spotify = Array.from(target.querySelectorAll(".provider-switch button")).find(
		(button) => button.textContent === "Spotify",
	);
	if (!(spotify instanceof HTMLButtonElement)) throw new Error("Missing Spotify button");
	spotify.click();
	await unmount(first);
	invoke.mockImplementation((command: string) =>
		Promise.resolve({
			status: "ready",
			data: command === "read_music_tracks" ? [currentTrack] : null,
		}),
	);
	const second = mount(MusicView, {
		target,
		props: { spotifyRevision: 201, onopenplayer: vi.fn(), onopensettings: vi.fn() },
	});
	await vi.waitFor(() => expect(target.textContent).toContain("Current account song"));
	completions.dispatchEvent(new Event("old"));
	await oldRead;
	await unmount(second);
	invoke.mockResolvedValue({ status: "failed", message: "Spotify returned 429" });
	const third = mount(MusicView, {
		target,
		props: { spotifyRevision: 201, onopenplayer: vi.fn(), onopensettings: vi.fn() },
	});
	await vi.waitFor(() => expect(target.textContent).toContain("Spotify returned 429"));
	expect(target.textContent).toContain("Current account song");
	expect(target.textContent).not.toContain("Old account response");
	await unmount(third);
	target.remove();
});
