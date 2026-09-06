import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { fromStore, writable } from "svelte/store";
import GameConnections from "../GameConnections.svelte";

const commands = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => commands);
let view: ReturnType<typeof mount> | null = null;
afterEach(async () => {
	if (view !== null) await unmount(view);
	view = null;
	commands.invoke.mockReset();
});

it("opens one miHoYo QR only after a verification login request", async () => {
	commands.invoke.mockImplementation((command: string) => {
		if (command === "read_game_connections")
			return Promise.resolve({
				status: "ready",
				data: { providers: [], mihoyo: [], bindings: [] },
			});
		if (command === "begin_game_login")
			return Promise.resolve({ status: "ready", data: { id: "qr", image: "", expiresAt: 1 } });
		if (command === "poll_game_login") return Promise.resolve({ status: "ready", data: "expired" });
		return Promise.resolve({ status: "ready", data: null });
	});
	const request = writable(false);
	const state = fromStore(request);
	const target = document.createElement("div");
	document.body.append(target);
	view = mount(GameConnections, {
		target,
		props: {
			get reconnectMihoyo() {
				return state.current;
			},
		},
	});
	await tick();
	expect(commands.invoke).not.toHaveBeenCalledWith("begin_game_login", { provider: "mihoyo" });
	request.set(true);
	await vi.waitFor(() =>
		expect(commands.invoke).toHaveBeenCalledWith("begin_game_login", { provider: "mihoyo" }),
	);
	await vi.waitFor(() =>
		expect(commands.invoke).toHaveBeenCalledWith("poll_game_login", {
			provider: "mihoyo",
			id: "qr",
		}),
	);
	expect(target.querySelector("dialog")?.open).toBe(true);
	expect(target.querySelector("dialog")?.textContent).toContain("Mihoyo mobile app");
	expect(target.querySelector("dialog")?.textContent).not.toContain("Genshin Impact mobile app");
	expect(
		commands.invoke.mock.calls.filter(([command]) => command === "begin_game_login"),
	).toHaveLength(1);
	expect(commands.invoke.mock.calls.some(([command]) => command === "read_game_notes")).toBe(false);
	await unmount(view);
	view = null;
	target.remove();
});

it("prefills Steam and retains edits made while saving", async () => {
	let finish: (result: { status: "ready"; data: null }) => void = () => {};
	commands.invoke.mockImplementation((command: string) => {
		if (command === "read_steam_settings")
			return Promise.resolve({
				status: "ready",
				data: { apiKey: "saved-key", steamId: "76561198000000000" },
			});
		if (command === "save_steam_connection")
			return new Promise((resolve) => {
				finish = resolve;
			});
		return Promise.resolve({
			status: "ready",
			data: { providers: ["steam"], mihoyo: [], bindings: [] },
		});
	});
	const target = document.createElement("div");
	document.body.append(target);
	view = mount(GameConnections, { target, props: { reconnectMihoyo: false } });
	const key = target.querySelector<HTMLInputElement>("#games-steam-key");
	const id = target.querySelector<HTMLInputElement>("#games-steam-id");
	const save = target.querySelector<HTMLButtonElement>('button[type="submit"]');
	if (!key || !id || !save) throw new Error("Steam form missing");
	await vi.waitFor(() => expect(key.value).toBe("saved-key"));
	expect(id.value).toBe("76561198000000000");
	expect(key.type).toBe("password");
	expect(save.disabled).toBe(true);
	key.value = "submitted-key";
	key.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	save.click();
	await vi.waitFor(() =>
		expect(commands.invoke).toHaveBeenCalledWith("save_steam_connection", {
			apiKey: "submitted-key",
			steamId: id.value,
		}),
	);
	key.value = "new-draft";
	key.dispatchEvent(new Event("input", { bubbles: true }));
	finish({ status: "ready", data: null });
	await vi.waitFor(() => expect(save.disabled).toBe(false));
	expect(key.value).toBe("new-draft");
	expect(id.value).toBe("76561198000000000");
	await unmount(view);
	view = null;
	target.remove();
});

it("prefills an untouched key after the ID is edited during loading", async () => {
	let finish: (result: {
		status: "ready";
		data: { apiKey: string; steamId: string };
	}) => void = () => {};
	commands.invoke.mockImplementation((command: string) => {
		if (command === "read_steam_settings")
			return new Promise((resolve) => {
				finish = resolve;
			});
		return Promise.resolve({
			status: "ready",
			data: { providers: ["steam"], mihoyo: [], bindings: [] },
		});
	});
	const target = document.createElement("div");
	view = mount(GameConnections, { target, props: { reconnectMihoyo: false } });
	await tick();
	const key = target.querySelector<HTMLInputElement>("#games-steam-key");
	const id = target.querySelector<HTMLInputElement>("#games-steam-id");
	if (!key || !id) throw new Error("Steam fields missing");
	id.value = "76561198000000001";
	id.dispatchEvent(new Event("input", { bubbles: true }));
	finish({ status: "ready", data: { apiKey: "stored-key", steamId: "76561198000000000" } });
	await vi.waitFor(() => expect(key.value).toBe("stored-key"));
	expect(id.value).toBe("76561198000000001");
	expect(target.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(false);
});
