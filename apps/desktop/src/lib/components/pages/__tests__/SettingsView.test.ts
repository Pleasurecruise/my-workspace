import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { fromStore, writable } from "svelte/store";
import type {
	ApiConfiguration,
	CommandResponse,
	CodexResets,
	ConfigurationStatus,
	GameConnections,
	NtfyConfig,
	NotionCalendar,
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
	notionCalendar: { status: "missing" },
	codexResets: { enabled: false },
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

async function setup(initialConfiguration: ConfigurationStatus = initial) {
	const target = document.createElement("div");
	const configuration = writable<ConfigurationStatus | null>(structuredClone(initialConfiguration));
	const snapshot = fromStore(configuration);
	const connectSpotify = vi
		.fn<(clientId: string) => Promise<CommandResponse<string>>>()
		.mockResolvedValue({ status: "ready", data: "Connected" });
	const save = vi
		.fn<
			(
				input:
					| ApiConfiguration
					| NtfyConfig
					| NotionCalendar
					| CodexResets
					| R2Configuration
					| UgosConfiguration
					| string,
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
				onsavenotion: save,
				onsavecodexresets: save,
				onsaventfy: save,
				onsaveapplock: save,
				onremoveapplock: vi.fn().mockResolvedValue({ status: "ready", data: "Removed" }),
				onconnectspotify: connectSpotify,
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
		const container = input.closest("form") ?? input.closest(".settings-row");
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
	return { target, configuration, save, connectSpotify, field, button, edit, submit };
}

it("prefills the Spotify Client ID, preserves edits and submits the chosen app", async () => {
	const form = await setup();
	const savedId = "0123456789abcdef0123456789abcdef";
	form.configuration.set({ ...initial, spotify: { status: "ready", data: savedId } });
	await tick();
	expect(form.field("spotify-client-id").value).toBe(savedId);
	await form.edit("spotify-client-id", "abcdef0123456789abcdef0123456789");
	form.configuration.set({ ...initial, spotify: { status: "ready", data: savedId } });
	await tick();
	expect(form.field("spotify-client-id").value).toBe("abcdef0123456789abcdef0123456789");
	const button = form.target.querySelector("#settings-music .settings-save-button");
	if (!(button instanceof HTMLButtonElement)) throw new Error("Missing Spotify connect button");
	form.connectSpotify.mockResolvedValueOnce({
		status: "failed",
		message: "Spotify authorization failed",
	});
	button.click();
	await tick();
	expect(form.connectSpotify).toHaveBeenLastCalledWith("abcdef0123456789abcdef0123456789");
	await vi.waitFor(() => expect(form.target.textContent).toContain("Spotify authorization failed"));
	await form.edit("spotify-client-id", "");
	button.click();
	await tick();
	expect(form.connectSpotify).toHaveBeenLastCalledWith("");
});

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

it("tracks the submitted Notion link while preserving edits made during saving", async () => {
	const { save, edit, submit, field, button } = await setup();
	expect(button("notion-calendar-url").disabled).toBe(true);
	let finish = (_response: CommandResponse<string>) => {};
	save.mockReturnValueOnce(
		new Promise((resolve) => {
			finish = resolve;
		}),
	);
	const first = "https://notion.so/calendar?v=248104cd477e80fdb757e945d38000bd";
	const second = "https://notion.so/calendar?v=248104cd477e80fdb757e945d38000be";
	await edit("notion-calendar-url", first);
	await submit("notion-calendar-url");
	await edit("notion-calendar-url", second);
	finish({ status: "ready", data: "notion-calendar" });
	await tick();
	expect(field("notion-calendar-url").value).toBe(second);
	await vi.waitFor(() => expect(button("notion-calendar-url").disabled).toBe(false));
	await edit("notion-calendar-url", first);
	expect(button("notion-calendar-url").disabled).toBe(true);
	await edit("notion-calendar-url", "");
	await submit("notion-calendar-url");
	expect(save).toHaveBeenLastCalledWith({ viewUrl: "" });
	expect(button("notion-calendar-url").disabled).toBe(true);
});

it("keeps Codex Resets failures retryable and saves enable and disable separately", async () => {
	const { field, button, submit, save, target } = await setup();
	const checkbox = field("codex-resets-enabled");
	expect(checkbox.checked).toBe(false);
	expect(button(checkbox.id).disabled).toBe(true);
	checkbox.checked = !checkbox.checked;
	checkbox.dispatchEvent(new Event("change", { bubbles: true }));
	await tick();
	save.mockResolvedValueOnce({ status: "failed", message: "Settings unavailable" });
	await submit(checkbox.id);
	expect(save).toHaveBeenLastCalledWith({ enabled: true });
	expect(target.textContent).toContain("Settings unavailable");
	expect(button(checkbox.id).disabled).toBe(false);
	await submit(checkbox.id);
	expect(button(checkbox.id).disabled).toBe(true);
	expect(target.textContent).not.toContain("Settings unavailable");
	checkbox.checked = !checkbox.checked;
	checkbox.dispatchEvent(new Event("change", { bubbles: true }));
	await tick();
	await submit(checkbox.id);
	expect(save).toHaveBeenLastCalledWith({ enabled: false });
	expect(button(checkbox.id).disabled).toBe(true);
});

it("preserves a Codex Resets draft changed during saving and configuration refresh", async () => {
	const { field, button, submit, save, configuration } = await setup();
	const checkbox = field("codex-resets-enabled");
	let finish: (response: CommandResponse<string>) => void = () => {
		throw new Error("Save not started");
	};
	save.mockReturnValueOnce(
		new Promise((resolve) => {
			finish = resolve;
		}),
	);
	checkbox.checked = !checkbox.checked;
	checkbox.dispatchEvent(new Event("change", { bubbles: true }));
	await tick();
	await submit(checkbox.id);
	expect(button(checkbox.id).disabled).toBe(true);
	checkbox.checked = !checkbox.checked;
	checkbox.dispatchEvent(new Event("change", { bubbles: true }));
	await tick();
	configuration.set({ ...initial, codexResets: { enabled: true } });
	finish({ status: "ready", data: "codex-resets" });
	await tick();
	expect(checkbox.checked).toBe(false);
	await vi.waitFor(() => expect(button(checkbox.id).disabled).toBe(false));
	await submit(checkbox.id);
	expect(save).toHaveBeenLastCalledWith({ enabled: false });
});

it("prefills an enabled Codex Resets subscription without marking it dirty", async () => {
	const { field, button } = await setup({
		...initial,
		codexResets: { enabled: true },
	});
	expect(field("codex-resets-enabled").checked).toBe(true);
	expect(button("codex-resets-enabled").disabled).toBe(true);
});
