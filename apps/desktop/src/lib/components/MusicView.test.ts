import { expect, it, vi } from "vite-plus/test";
import { mount, unmount } from "svelte";
import type { CommandResponse, MusicLyrics, MusicPlayback, MusicTrack } from "../consumer";
import MusicView from "./MusicView.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: vi.fn() }));

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
  let resolvePoll!: (response: CommandResponse<MusicPlayback>) => void;
  const poll = new Promise<CommandResponse<MusicPlayback>>((resolve) => {
    resolvePoll = resolve;
  });
  let resolvePlay!: (response: CommandResponse<string>) => void;
  const play = new Promise<CommandResponse<string>>((resolve) => {
    resolvePlay = resolve;
  });
  let resolveOldLyrics!: (response: CommandResponse<MusicLyrics>) => void;
  const oldLyrics = new Promise<CommandResponse<MusicLyrics>>((resolve) => {
    resolveOldLyrics = resolve;
  });
  let resolveNewLyrics!: (response: CommandResponse<MusicLyrics>) => void;
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
  target.querySelector<HTMLButtonElement>('[aria-label="Next song"]')!.click();
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
  target.querySelector<HTMLButtonElement>('[aria-label="Previous song"]')!.click();
  await vi.waitFor(() => expect(target.textContent).toContain("Playback rejected"));
  expect(target.querySelector(".now-playing strong")?.textContent).toBe("Track B");
  expect(lyricReads).toBe(2);
  await unmount(view);
  target.remove();
  vi.useRealTimers();
});
