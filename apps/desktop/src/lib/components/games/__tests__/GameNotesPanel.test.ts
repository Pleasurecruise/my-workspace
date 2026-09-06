import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, unmount } from "svelte";
import type { Game, GameNotesResponse } from "../../../consumer";
import GameNotesPanel from "../GameNotesPanel.svelte";

const commands = vi.hoisted(() => ({ invoke: vi.fn() }));
const events = vi.hoisted(() => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("@tauri-apps/api/core", () => commands);
vi.mock("@tauri-apps/api/event", () => events);
let view: ReturnType<typeof mount> | null = null;
afterEach(async () => {
	if (view !== null) await unmount(view);
	view = null;
	commands.invoke.mockReset();
	events.listen.mockClear();
});

it.each([1034, 10035, 10041])(
	"keeps code %s cached on close until an explicit refresh",
	async (code) => {
		let result: GameNotesResponse = {
			status: "verificationRequired",
			code,
			message:
				"miHoYo requires security verification. Complete verification, then use the refresh icon.",
		};
		commands.invoke.mockImplementation((command: string) =>
			Promise.resolve(command === "read_game_notes" ? result : { status: "ready", data: true }),
		);
		const target = document.createElement("div");
		view = mount(GameNotesPanel, { target, props: { game: "starRail" } });
		await vi.waitFor(() => expect(target.textContent).toContain("security verification"));
		expect(target.textContent).not.toContain(String(code));
		const verify = target.querySelectorAll("button")[1];
		if (!(verify instanceof HTMLButtonElement)) throw new Error("Missing verification action");
		verify.click();
		await vi.waitFor(() =>
			expect(commands.invoke).toHaveBeenCalledWith("verify_game", { game: "starRail" }),
		);
		expect(verify.disabled).toBe(true);
		result = {
			status: "ready",
			data: {
				account: {
					game: "starRail",
					uid: "100",
					region: "prod_gf_cn",
					roleId: "100",
					name: "Trailblazer",
				},
				sampledAt: 1,
				meters: [],
				tasks: [],
			},
		};
		const closed = events.listen.mock.calls.find(([name]) => name === "game-verification-closed");
		if (!closed) throw new Error("Missing close subscription");
		closed[1]({ payload: "genshin" });
		expect(commands.invoke.mock.calls.filter(([name]) => name === "read_game_notes")).toHaveLength(
			1,
		);
		closed[1]({ payload: "starRail" });
		await vi.waitFor(() => expect(verify.disabled).toBe(false));
		expect(commands.invoke.mock.calls.filter(([name]) => name === "read_game_notes")).toHaveLength(
			1,
		);
		const refresh = target.querySelector("button");
		if (!(refresh instanceof HTMLButtonElement)) throw new Error("Missing refresh action");
		expect(refresh.textContent?.trim()).toBe("");
		expect(refresh.querySelector("svg")).not.toBeNull();
		expect(refresh.getAttribute("aria-label")).toBe("Refresh daily status");
		refresh.click();
		await vi.waitFor(() =>
			expect(commands.invoke).toHaveBeenCalledWith("read_game_notes", {
				game: "starRail",
				refresh: true,
			}),
		);
		await vi.waitFor(() => expect(target.textContent).toContain("Trailblazer"));
		expect(target.querySelector('[role="alert"]')).toBeNull();
		expect(target.querySelectorAll("button")).toHaveLength(1);
	},
);

it("keeps verification available after the window fails to open", async () => {
	commands.invoke.mockImplementation((command: string) =>
		Promise.resolve(
			command === "read_game_notes"
				? { status: "verificationRequired", code: 5003, message: "Restricted (code 5003)" }
				: { status: "failed", message: "Could not open verification" },
		),
	);
	const target = document.createElement("div");
	view = mount(GameNotesPanel, { target, props: { game: "starRail" } });
	await vi.waitFor(() => expect(target.textContent).toContain("5003"));
	const verify = target.querySelectorAll("button")[1];
	if (!(verify instanceof HTMLButtonElement)) throw new Error("Missing verification action");
	verify.click();
	await vi.waitFor(() => expect(target.textContent).toContain("Could not open verification"));
	expect(verify.disabled).toBe(false);
});

it.each([
	{ game: "genshin", label: "Daily commissions", total: 4, current: 0 },
	{ game: "genshin", label: "Daily commissions", total: 4, current: 2 },
	{ game: "genshin", label: "Daily commissions", total: 4, current: 4 },
	{ game: "genshin", label: "Expeditions", total: 5, current: 5 },
	{ game: "genshin", label: "Expeditions", total: 5, current: 2 },
	{ game: "genshin", label: "Weekly discounts remaining", total: 3, current: 3 },
	{ game: "genshin", label: "Weekly discounts remaining", total: 3, current: 0 },
	{ game: "starRail", label: "Assignments", total: 4, current: 0 },
	{ game: "starRail", label: "Assignments", total: 4, current: 2 },
	{ game: "starRail", label: "Assignments", total: 4, current: 4 },
	{ game: "starRail", label: "Daily training", total: 5, current: 0 },
	{ game: "starRail", label: "Daily training", total: 5, current: 3 },
	{ game: "starRail", label: "Daily training", total: 5, current: 5 },
] satisfies Array<{ game: Game; label: string; total: number; current: number }>)(
	"renders $game $label ($current/$total) as accessible stars",
	async ({ game, label, current, total }) => {
		commands.invoke.mockResolvedValue({
			status: "ready",
			data: {
				account: { game, uid: "100", name: "Traveler", region: "cn_gf01", roleId: "100" },
				sampledAt: 1,
				meters: [{ game: "genshin", label: "Original Resin", current: 80, max: 200, fullAt: null }],
				tasks: [{ label, value: `${current} / ${total}`, progress: { current, total } }],
			},
		});
		const target = document.createElement("div");
		view = mount(GameNotesPanel, { target, props: { game } });
		await vi.waitFor(() => expect(target.querySelector('[role="img"]')).not.toBeNull());
		const stars = target.querySelector('[role="img"]');
		expect(stars?.getAttribute("aria-label")).toBe(`${label}: ${current} of ${total}`);
		expect(stars?.querySelectorAll("svg")).toHaveLength(total);
		expect(stars?.querySelectorAll('svg[fill="currentColor"]')).toHaveLength(current);
		expect(stars?.querySelectorAll('svg[fill="none"]')).toHaveLength(total - current);
		expect(target.textContent).not.toContain(`${current} / ${total}`);
		expect(target.querySelector("section")?.classList.contains("compact")).toBe(true);
		expect(commands.invoke).toHaveBeenCalledTimes(1);
	},
);

it("does not claim success when verification returns no authorization", async () => {
	commands.invoke.mockImplementation((command: string) =>
		Promise.resolve(
			command === "read_game_notes"
				? { status: "verificationRequired", code: 10041, message: "Security verification required" }
				: { status: "failed", message: "miHoYo did not authorize game-record access." },
		),
	);
	const target = document.createElement("div");
	view = mount(GameNotesPanel, { target, props: { game: "starRail" } });
	await vi.waitFor(() => expect(target.querySelectorAll("button")).toHaveLength(2));
	target.querySelectorAll("button")[1]?.click();
	await vi.waitFor(() =>
		expect(target.querySelector('[role="alert"]')?.textContent).toContain("did not authorize"),
	);
	expect(target.querySelector('[role="status"]')).toBeNull();
	expect(target.querySelectorAll("button")).toHaveLength(2);
	expect(commands.invoke.mock.calls.filter(([name]) => name === "read_game_notes")).toHaveLength(1);
	expect(target.querySelector("button")?.disabled).toBe(false);
});

it("replaces a cached verification error with a manual-refresh notice on completion", async () => {
	commands.invoke.mockResolvedValue({
		status: "verificationRequired",
		code: 10041,
		message: "Security verification required",
	});
	const target = document.createElement("div");
	view = mount(GameNotesPanel, { target, props: { game: "starRail" } });
	await vi.waitFor(() => expect(target.querySelector('[role="alert"]')).not.toBeNull());
	const updated = events.listen.mock.calls.find(([name]) => name === "game-notes-updated");
	if (!updated) throw new Error("Missing note subscription");
	updated[1]({
		payload: {
			game: "starRail",
			result: {
				status: "refreshRequired",
				message: "Verification accepted. Use the refresh icon to check game-record access.",
			},
		},
	});
	await vi.waitFor(() =>
		expect(target.querySelector('[role="status"]')?.textContent).toContain("Verification accepted"),
	);
	expect(target.querySelector('[role="alert"]')).toBeNull();
	expect(target.querySelectorAll("button")).toHaveLength(1);
	expect(commands.invoke).toHaveBeenCalledTimes(1);
});
