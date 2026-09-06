import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, GachaArchive } from "../../../consumer";
import GachaPanel from "../GachaPanel.svelte";

const commands = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => commands);
let view: ReturnType<typeof mount> | null = null;
afterEach(async () => {
	if (view !== null) await unmount(view);
	view = null;
});

it("preserves the archived account after failed sync", async () => {
	const archive: GachaArchive = {
		game: "genshin",
		uid: "100",
		accounts: [{ uid: "100", name: "Traveler" }],
		total: 42,
		added: 0,
		syncedAt: 1788696000,
		firstRecord: null,
		lastRecord: null,
		pools: [],
		recent: [],
		official: null,
	};
	const complete = vi.fn<(value: CommandResponse<GachaArchive>) => void>();
	const pending = new Promise<CommandResponse<GachaArchive>>((resolve) =>
		complete.mockImplementation(resolve),
	);
	commands.invoke.mockImplementation((command: string) => {
		if (command === "sync_gacha_archive") return pending;
		return Promise.resolve({ status: "ready", data: archive });
	});
	const target = document.createElement("div");
	view = mount(GachaPanel, { target, props: { game: "genshin" } });
	await vi.waitFor(() => expect(target.textContent).toContain("42 archived pulls"));
	const sync = Array.from(target.querySelectorAll("button")).find(
		(button) => button.getAttribute("aria-label") === "Sync history",
	);
	if (sync === undefined) throw new Error("Missing archive controls");
	expect(target.querySelectorAll("button")).toHaveLength(1);
	sync.click();
	await tick();
	expect(sync.disabled).toBe(true);
	complete({ status: "failed", message: "Provider history unavailable" });
	await vi.waitFor(() => expect(target.textContent).toContain("Provider history unavailable"));
	expect(target.textContent).toContain("42 archived pulls");
	expect(sync.disabled).toBe(false);
	expect(target.querySelector('[title="Archive · 100"]')).not.toBeNull();
});

it("shows official Star Rail totals without inventing lower-rarity pull records", async () => {
	commands.invoke.mockReset();
	const data: GachaArchive = {
		game: "starRail",
		uid: "100",
		accounts: [{ uid: "100", name: "Trailblazer" }],
		total: 0,
		added: 0,
		syncedAt: 1788696000,
		firstRecord: null,
		lastRecord: null,
		pools: [],
		recent: [],
		official: {
			pools: [
				{
					id: "11",
					name: "Character event",
					total: 150,
					sinceHighRarity: 30,
					fiveStars: [{ id: "one", itemId: 1, name: "Test character", pulls: 60, isUp: true }],
				},
			],
		},
	};
	commands.invoke.mockResolvedValue({ status: "ready", data });
	const target = document.createElement("div");
	view = mount(GachaPanel, { target, props: { game: "starRail" } });
	await vi.waitFor(() => expect(target.textContent).toContain("150 reported pulls"));
	expect(target.textContent).toContain("Individual 3★ and 4★ records are not provided");
	expect(target.textContent).toContain("Test character");
	expect(target.textContent).not.toContain("No archived pulls yet");
	expect(target.querySelectorAll("svg[role=img]")).toHaveLength(1);
	expect(target.querySelector("svg[role=img]")?.getAttribute("aria-label")).toContain(
		"Character event: 150",
	);
	expect(target.querySelector("circle.segment")?.getAttribute("stroke-dasharray")).toBe("100 100");
	expect(target.querySelectorAll("button")).toHaveLength(1);
	expect(commands.invoke).toHaveBeenCalledTimes(1);
	expect(commands.invoke).toHaveBeenCalledWith("read_gacha_archive", {
		game: "starRail",
		uid: null,
	});
});
