import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { fromStore, writable } from "svelte/store";
import type {
	ApiConfiguration,
	CommandResponse,
	ConfigurationStatus,
	GameConnections,
	NtfyConfig,
	R2Configuration,
	UgosConfiguration,
} from "../../../consumer";
import SettingsView from "../SettingsView.svelte";

vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn().mockImplementation((command: string) => {
		if (command === "read_steam_settings") return Promise.resolve({ status: "ready", data: null });
		if (command === "read_game_connections") {
			const data: GameConnections = { providers: [], mihoyo: [], bindings: [] };
			return Promise.resolve({ status: "ready", data });
		}
		return Promise.resolve({ status: "ready", data: [] });
	}),
}));

const initial: ConfigurationStatus = {
	ugos: { status: "ready", data: { username: "user", password: "password" } },
	r2: { status: "ready", data: { accessKeyId: "access", secretAccessKey: "secret" } },
	api: {
		memos: { status: "ready", data: "memos-key" },
		moment: { status: "ready", data: "moment-key" },
		knowledge: { status: "ready", data: "knowledge-key" },
	},
	ntfy: { status: "ready", data: { token: "ntfy-token", development: false } },
	ntfyDev: false,
	appLock: { status: "ready", data: "lock-password" },
	appLockDev: false,
	spotify: { status: "missing" },
	qqMusic: { status: "missing" },
	publication: { telegram: false, x: false },
};

const views: ReturnType<typeof mount>[] = [];
afterEach(async () => {
	for (const view of views.splice(0)) await unmount(view);
});

async function setup() {
	const target = document.createElement("div");
	const configuration = writable<ConfigurationStatus | null>(structuredClone(initial));
	const snapshot = fromStore(configuration);
	const save = vi
		.fn<
			(
				input: ApiConfiguration | NtfyConfig | R2Configuration | UgosConfiguration | string,
			) => Promise<CommandResponse<string>>
		>()
		.mockResolvedValue({ status: "ready", data: "Saved" });
	views.push(
		mount(SettingsView, {
			target,
			props: {
				reconnectMihoyo: false,
				get configuration() {
					return snapshot.current;
				},
				error: null,
				onsaveugos: save,
				onsaver2: save,
				onsaveapi: save,
				onsaventfy: save,
				onsaveapplock: save,
				onremoveapplock: vi.fn().mockResolvedValue({ status: "ready", data: "Removed" }),
				onconnectspotify: vi.fn().mockResolvedValue({ status: "ready", data: "Connected" }),
				onbeginqq: vi.fn(),
				onpollqq: vi.fn(),
				oncancelqq: vi.fn(),
				onconfigurationchanged: vi.fn(),
			},
		}),
	);
	await tick();
	function field(id: string) {
		const input = target.querySelector(`#${id}`);
		if (!(input instanceof HTMLInputElement)) throw new Error(`Missing input: ${id}`);
		return input;
	}
	function button(id: string) {
		const input = field(id);
		const container = input.closest("form") ?? input.closest(".setting-row");
		if (container === null) throw new Error(`Missing form or settings row: ${id}`);
		for (const item of container.querySelectorAll("button")) {
			if (
				item instanceof HTMLButtonElement &&
				(item.textContent === "Save" || item.textContent === "Saving…")
			)
				return item;
		}
		throw new Error(`Missing Save button: ${id}`);
	}
	async function edit(id: string, value: string) {
		field(id).value = value;
		field(id).dispatchEvent(new Event("input", { bubbles: true }));
		await tick();
	}
	async function submit(id: string) {
		const form = field(id).closest("form");
		if (form) form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
		else button(id).click();
		await tick();
	}
	return { target, configuration, save, field, button, edit, submit };
}

it("preserves unsaved fields when switching settings categories", async () => {
	const { target, edit, field, button } = await setup();
	const general = target.querySelector("#settings-general");
	const services = target.querySelector("#settings-services");
	expect(general?.hasAttribute("hidden")).toBe(false);
	expect(services?.hasAttribute("hidden")).toBe(true);
	await edit("memos-api-key", "unsaved-key");
	for (const category of ["services", "music", "services"]) {
		const navigation = target.querySelector(`button[aria-controls="settings-${category}"]`);
		if (!(navigation instanceof HTMLButtonElement)) throw new Error("Missing category");
		navigation.click();
		await tick();
		expect(navigation.getAttribute("aria-pressed")).toBe("true");
		if (category === "music") continue;
		expect(services?.hasAttribute("hidden")).toBe(false);
		expect(field("memos-api-key").value).toBe("unsaved-key");
	}
	expect(field("memos-api-key").value).toBe("unsaved-key");
	expect(button("memos-api-key").disabled).toBe(false);
	expect(general?.hasAttribute("hidden")).toBe(true);
});

it.each([
	["ugos-username", "user"],
	["ugos-password", "password"],
	["r2-access-key", "access"],
	["r2-secret-key", "secret"],
	["memos-api-key", "memos-key"],
	["moment-api-key", "moment-key"],
	["knowledge-api-key", "knowledge-key"],
	["ntfy-token", "ntfy-token"],
	["app-lock-password", "lock-password"],
])("enables %s only for complete, changed input and resets after saving", async (id, original) => {
	const form = await setup();
	expect(form.button(id).disabled).toBe(true);
	await form.submit(id);
	expect(form.save).not.toHaveBeenCalled();
	await form.edit(id, `${original}-updated`);
	expect(form.button(id).disabled).toBe(false);
	await form.edit(id, original);
	expect(form.button(id).disabled).toBe(true);
	await form.edit(id, "");
	expect(form.button(id).disabled).toBe(true);
	await form.edit(id, `${original}-updated`);
	await form.submit(id);
	expect(form.save).toHaveBeenCalledOnce();
	expect(form.button(id).disabled).toBe(true);
});

it("keeps pending saves independent and preserves later edits across configuration refreshes", async () => {
	const form = await setup();
	const completions = new EventTarget();
	const memos = new Promise<CommandResponse<string>>((resolve) => {
		completions.addEventListener("memos", () => resolve({ status: "ready", data: "Saved" }), {
			once: true,
		});
	});
	const moment = new Promise<CommandResponse<string>>((resolve) => {
		completions.addEventListener(
			"moment",
			() => resolve({ status: "failed", message: "Could not save" }),
			{ once: true },
		);
	});
	form.save.mockReturnValueOnce(memos).mockReturnValueOnce(moment);
	await form.edit("memos-api-key", "memos-submitted");
	await form.edit("moment-api-key", "moment-submitted");
	await form.submit("memos-api-key");
	expect(form.button("memos-api-key").disabled).toBe(true);
	expect(form.button("moment-api-key").disabled).toBe(false);
	await form.submit("moment-api-key");
	await form.edit("memos-api-key", "memos-next-draft");
	await form.edit("ntfy-token", "ntfy-draft");
	form.configuration.set({
		...structuredClone(initial),
		api: {
			...initial.api,
			memos: { status: "ready", data: "memos-submitted" },
		},
	});
	await tick();
	completions.dispatchEvent(new Event("memos"));
	await tick();
	expect(form.field("memos-api-key").value).toBe("memos-next-draft");
	expect(form.field("ntfy-token").value).toBe("ntfy-draft");
	await vi.waitFor(() => expect(form.button("memos-api-key").disabled).toBe(false));
	expect(form.button("moment-api-key").disabled).toBe(true);
	completions.dispatchEvent(new Event("moment"));
	await tick();
	await vi.waitFor(() => expect(form.button("moment-api-key").disabled).toBe(false));
	await form.edit("memos-api-key", "memos-submitted");
	expect(form.button("memos-api-key").disabled).toBe(true);
});
