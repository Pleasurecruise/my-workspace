import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, SteamGames } from "../../../consumer";
import SteamGamesPanel from "../SteamGamesPanel.svelte";

const commands = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn(), openUrl: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: commands.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: commands.listen }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: commands.openUrl }));
let view: ReturnType<typeof mount> | null = null;
afterEach(async () => {
	if (view !== null) await unmount(view);
	view = null;
	vi.resetAllMocks();
});
const snapshot: SteamGames = {
	name: "Player",
	profileUrl: "https://steamcommunity.com/id/example",
	state: 1,
	playing: "Portal",
	ownedGames: 12,
	recentCount: 2,
	recentMinutes: 180,
	totalMinutes: 1200,
	playedGames: 8,
	sampledAt: 100,
	recent: [{ appId: 400, name: "Portal", playtimeForever: 600, playtime2weeks: 180 }],
	mostPlayed: [{ appId: 620, name: "Portal 2", playtimeForever: 600, playtime2weeks: null }],
};
it("shows activity and keeps settled data when refresh fails", async () => {
	commands.listen.mockResolvedValue(() => {});
	commands.invoke
		.mockResolvedValueOnce({ status: "ready", data: snapshot })
		.mockResolvedValueOnce({ status: "failed", message: "Steam unavailable" });
	const target = document.createElement("div");
	view = mount(SteamGamesPanel, { target });
	await vi.waitFor(() => expect(target.textContent).toContain("Portal 2"));
	expect(target.textContent).toContain("Online");
	expect(target.textContent).toContain("3 h");
	expect(target.textContent).toContain("20 h");
	target.querySelector<HTMLButtonElement>("header button")?.click();
	await vi.waitFor(() => expect(target.textContent).toContain("Steam unavailable"));
	expect(target.textContent).toContain("Portal 2");
});
it("preserves a newer event over a delayed initial read and labels hidden data", async () => {
	let finish: (result: CommandResponse<SteamGames>) => void = () => {};
	let emit: (event: { payload: CommandResponse<SteamGames> }) => void = () => {};
	commands.invoke.mockImplementation(
		() =>
			new Promise((resolve) => {
				finish = resolve;
			}),
	);
	commands.listen.mockImplementation((_event: string, callback: typeof emit) => {
		emit = callback;
		return Promise.resolve(() => {});
	});
	const target = document.createElement("div");
	view = mount(SteamGamesPanel, { target });
	await tick();
	emit({
		payload: {
			status: "ready",
			data: {
				...snapshot,
				name: "New player",
				ownedGames: null,
				recentCount: null,
				recentMinutes: null,
				totalMinutes: null,
				playedGames: null,
				recent: [],
				mostPlayed: [],
			},
		},
	});
	finish({ status: "ready", data: snapshot });
	await vi.waitFor(() => expect(target.textContent).toContain("New player"));
	expect(target.textContent).toContain("Game details unavailable");
	expect(target.textContent).not.toContain("Portal 2");
	expect(target.textContent).not.toContain("0 h");
});
