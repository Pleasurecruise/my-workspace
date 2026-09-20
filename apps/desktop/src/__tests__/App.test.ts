import { beforeEach, afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { listen } from "@tauri-apps/api/event";
import App from "../App.svelte";
import type {
	ChannelView,
	ConfigurationStatus,
	CommandResponse,
	InitialViews,
	MemoTagCount,
	MemoView,
	SshOutput,
} from "../lib/consumer";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
	invoke,
	convertFileSrc: (path: string) => path,
	Channel: class {
		onmessage = (_message: SshOutput) => {};
	},
}));
vi.mock("@xterm/xterm", () => ({
	Terminal: class {
		cols = 80;
		rows = 24;
		loadAddon() {}
		open() {}
		reset() {}
		focus() {}
		write() {}
		dispose() {}
		onData() {}
		onBinary() {}
	},
}));
vi.mock("@xterm/addon-fit", () => ({
	FitAddon: class {
		fit() {}
	},
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("../lib/theme", () => ({ initTheme: () => false, applyTheme: vi.fn() }));

function deferred<T>() {
	let settle: (value: T) => void = () => {
		throw new Error("Promise not initialized");
	};
	const promise = new Promise<T>((resolve) => {
		settle = resolve;
	});
	return { promise, resolve: (value: T) => settle(value) };
}

const configuration: ConfigurationStatus = {
	ugos: { status: "missing" },
	r2: { status: "missing" },
	api: {
		memos: { status: "ready", data: "test-memos" },
		moment: { status: "ready", data: "test-moment" },
		knowledge: { status: "ready", data: "test-knowledge" },
	},
	ntfy: { status: "ready", data: { token: "test-ntfy", development: false } },
	ntfyDev: false,
	notionCalendar: { status: "missing" },
	codexResets: { enabled: false },
	appLock: { status: "missing" },
	appLockDev: false,
	spotify: { status: "missing" },
	qqMusic: { status: "missing" },
	publication: { telegram: false, x: false },
};
const memos: ChannelView = {
	channel: "memos",
	memos: [],
	nextCursor: null,
};
beforeEach(() => {
	vi.stubGlobal("localStorage", { getItem: () => null, setItem: vi.fn() });
	vi.mocked(listen)
		.mockReset()
		.mockResolvedValue(() => {});
});
const views: ReturnType<typeof mount>[] = [];
afterEach(async () => {
	for (const view of views.splice(0)) await unmount(view);
	document.body.replaceChildren();
	invoke.mockReset();
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

function button(target: HTMLElement, label: string, selector = "button") {
	const found = Array.from(target.querySelectorAll<HTMLButtonElement>(selector)).find(
		(item) => item.textContent?.trim() === label,
	);
	if (!found) throw new Error(`Button ${label} missing`);
	return found;
}

function setupCommands() {
	invoke.mockImplementation(async (command: string) => {
		if (command === "initialize_views") return new Promise(() => {});
		if (command === "read_app_lock") return false;
		if (command === "read_photo_metadata")
			return { status: "ready", data: { capturedAt: null, geo: null } };
		if (command === "read_configuration") return { status: "ready", data: configuration };
		if (command === "check_for_update") return { status: "ready", data: null };
		if (command === "read_notifications") return { status: "ready", data: [] };
		if (command === "read_channel") return { status: "ready", data: memos };
		return { status: "failed", message: "Not configured in this test" };
	});
}

it("collapses the sidebar and restores its saved width", async () => {
	setupCommands();
	const saved = new Map<string, string>();
	vi.stubGlobal("localStorage", {
		getItem: (key: string) => saved.get(key) ?? null,
		setItem: (key: string, value: string) => saved.set(key, value),
	});
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(App, { target });
	await tick();
	const handle = target.querySelector<HTMLButtonElement>(".sidebar-resizer");
	if (!handle) throw new Error("Missing resize handle");
	handle.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true }));
	await tick();
	expect(target.querySelector("aside")?.classList.contains("compact")).toBe(true);
	expect(saved.get("vesper.sidebar.width")).toBe("64");
	expect(button(target, "Dashboard", "nav button").getAttribute("aria-label")).toBe("Dashboard");
	expect(target.querySelector(".user-profile img")).not.toBeNull();
	await unmount(view);
	views.push(mount(App, { target }));
	await tick();
	expect(target.querySelector("aside")?.classList.contains("compact")).toBe(true);
	const restored = target.querySelector<HTMLButtonElement>(".sidebar-resizer");
	if (!restored) throw new Error("Missing resize handle");
	restored.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
	await tick();
	expect(target.querySelector("aside")?.classList.contains("compact")).toBe(false);
	expect(saved.get("vesper.sidebar.width")).toBe("360");
});

it("renders ready tags before initialization and the other source finish; failed tags can retry", async () => {
	setupCommands();
	const initial = deferred<InitialViews>();
	const other = deferred<CommandResponse<string[]>>();
	const normal = invoke.getMockImplementation();
	if (!normal) throw new Error("Missing command mock");
	let reads = 0;
	invoke.mockImplementation((command: string) => {
		if (command === "initialize_views") return initial.promise;
		if (command === "read_moment_tags") return other.promise;
		if (command === "read_memo_tags")
			return Promise.resolve(
				++reads === 1
					? { status: "failed", message: "Tag timeout" }
					: { status: "ready", data: [{ name: "recovered", count: 2 }] },
			);
		if (!normal) throw new Error("Missing command mock");
		return normal(command);
	});
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() => expect(button(target, "Memos", "nav button")).toBeTruthy());
	await tick();
	await vi.waitFor(() => expect(reads).toBe(1));
	// Entering the page retries the startup failure; both unrelated requests remain pending.
	button(target, "Memos", "nav button").click();
	await vi.waitFor(() =>
		expect(target.querySelector('[aria-label="Memo tags"]')?.textContent).toContain("recovered"),
	);
	expect(target.textContent).not.toContain("Tags unavailable");
	invoke.mockImplementation((command: string) =>
		command === "read_memo_tags"
			? Promise.resolve({ status: "failed", message: "Tag timeout" })
			: normal(command),
	);
	button(target, "Dashboard", "nav button").click();
	await tick();
	button(target, "Memos", "nav button").click();
	await vi.waitFor(() => expect(target.textContent).toContain("Tags unavailable: Tag timeout"));
	expect(target.querySelector('[aria-label="Memo tags"]')?.textContent).toContain("recovered");
	invoke.mockResolvedValue({ status: "ready", data: [] });
	button(target, "Retry tags").click();
	await vi.waitFor(() => expect(target.textContent).not.toContain("Tags unavailable"));
	expect(target.querySelector('[aria-label="Memo tags"]')).toBeNull();
});

it("preserves a completed write after navigation against older tags and startup data", async () => {
	setupCommands();
	const initial = deferred<InitialViews>();
	const oldTags = deferred<CommandResponse<MemoTagCount[]>>();
	const save = deferred<CommandResponse<MemoView>>();
	const normal = invoke.getMockImplementation();
	if (!normal) throw new Error("Missing command mock");
	let reads = 0;
	invoke.mockImplementation((command: string) => {
		if (command === "initialize_views") return initial.promise;
		if (command === "read_memo_tags")
			return ++reads === 1
				? oldTags.promise
				: Promise.resolve({ status: "ready", data: [{ name: "new", count: 1 }] });
		if (command === "read_moment_tags") return Promise.resolve({ status: "ready", data: [] });
		if (command === "create_memo") return save.promise;
		if (!normal) throw new Error("Missing command mock");
		return normal(command);
	});
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() => expect(button(target, "Memos", "nav button")).toBeTruthy());
	await tick();
	button(target, "Memos", "nav button").click();
	await vi.waitFor(() => expect(target.querySelector("textarea")).not.toBeNull());
	const editor = target.querySelector("textarea");
	if (!editor) throw new Error("Memo editor missing");
	editor.value = "Saved across navigation #new";
	editor.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	button(target, "Save").click();
	await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("create_memo", expect.anything()));
	button(target, "Dashboard", "nav button").click();
	await tick();
	save.resolve({
		status: "ready",
		data: {
			id: "created",
			r2Key: "memo/created",
			content: "Saved across navigation #new",
			html: "<p>Saved across navigation</p>",
			tags: ["new"],
			createdAt: "2026-09-06T00:00:00Z",
			updatedAt: "2026-09-06T00:00:00Z",
			visibility: "private",
			pinned: false,
			favorite: false,
			archived: false,
			metadataComplete: true,
		},
	});
	await vi.waitFor(() => expect(reads).toBe(2));
	oldTags.resolve({ status: "ready", data: [{ name: "old", count: 4 }] });
	initial.resolve({
		memos: { status: "ready", data: memos },
		moment: { status: "failed", message: "Not configured" },
		knowledge: { status: "failed", message: "Not configured" },
	});
	await tick();
	button(target, "Memos", "nav button").click();
	await vi.waitFor(() =>
		expect(target.querySelector('[aria-label="Memo tags"]')?.textContent).toContain("new"),
	);
	expect(target.querySelector('[aria-label="Memo tags"]')?.textContent).not.toContain("old");
	expect(target.querySelector('main [aria-label="Memos"]')?.textContent).toContain(
		"Saved across navigation",
	);
});

it.each([false, true])(
	"checks the upload session before sending file bytes (credential change: %s)",
	async (changeCredentials) => {
		setupCommands();
		const initial = deferred<InitialViews>();
		const bytes = deferred<ArrayBuffer>();
		const normal = invoke.getMockImplementation();
		if (!normal) throw new Error("Missing command mock");
		let tagReads = 0;
		invoke.mockImplementation((command: string) => {
			if (command === "initialize_views") return initial.promise;
			if (command === "read_channel")
				return Promise.resolve({
					status: "ready",
					data: { channel: "moment", photos: [], total: 0 },
				});
			if (command === "read_moment_tags") {
				tagReads += 1;
				return Promise.resolve({ status: "ready", data: [] });
			}
			if (command === "save_api_configuration")
				return Promise.resolve({ status: "ready", data: "Saved" });
			return normal(command);
		});
		vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:test-photo");
		vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
		const target = document.createElement("div");
		document.body.append(target);
		views.push(mount(App, { target }));
		await vi.waitFor(() => expect(button(target, "Memos", "nav button")).toBeTruthy());
		await tick();
		button(target, "Moment", "nav button").click();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Upload photos"]')).not.toBeNull(),
		);
		const upload = target.querySelector<HTMLButtonElement>('[aria-label="Upload photos"]');
		if (!upload) throw new Error("Upload action missing");
		upload.click();
		await tick();
		const file = new File(["photo"], "photo.png", { type: "image/png" });
		const read = vi
			.spyOn(file, "arrayBuffer")
			.mockResolvedValueOnce(new ArrayBuffer(4))
			.mockReturnValue(bytes.promise);
		const input = target.querySelector<HTMLInputElement>('input[type="file"]');
		if (!input) throw new Error("File input missing");
		Object.defineProperty(input, "files", {
			configurable: true,
			value: { item: (index: number) => (index === 0 ? file : null) },
		});
		input.dispatchEvent(new Event("change", { bubbles: true }));
		await tick();
		await vi.waitFor(() => expect(button(target, "Publish").disabled).toBe(false));
		button(target, "Publish").click();
		await vi.waitFor(() => expect(read).toHaveBeenCalledTimes(2));
		button(target, changeCredentials ? "Settings" : "Dashboard", "nav button").click();
		await tick();
		if (changeCredentials) {
			const key = target.querySelector<HTMLInputElement>("#moment-api-key");
			if (!key) throw new Error("Moment credential input missing");
			key.value = "replacement-key";
			key.dispatchEvent(new Event("input", { bubbles: true }));
			await tick();
			const row = key.closest(".settings-row");
			if (!(row instanceof HTMLElement)) throw new Error("Credential row missing");
			const previousReads = tagReads;
			button(row, "Save").click();
			await vi.waitFor(() => expect(tagReads).toBeGreaterThan(previousReads));
		}
		bytes.resolve(new ArrayBuffer(4));
		await bytes.promise;
		await tick();
		if (changeCredentials)
			expect(invoke).not.toHaveBeenCalledWith("create_photo", expect.anything());
		else
			await vi.waitFor(() =>
				expect(invoke).toHaveBeenCalledWith(
					"create_photo",
					expect.objectContaining({ source: [0, 0, 0, 0] }),
				),
			);
	},
);

it("shows the destination's loading structure while its content request is pending", async () => {
	setupCommands();
	const normal = invoke.getMockImplementation();
	if (!normal) throw new Error("Missing command mock");
	invoke.mockImplementation((command: string) => {
		if (command === "initialize_views" || command === "read_channel") return new Promise(() => {});
		return normal(command);
	});
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() => expect(button(target, "Memos", "nav button")).toBeTruthy());
	await tick();
	for (const [label, shape] of [
		["Memos", ".composer"],
		["Moment", ".photos"],
		["Knowledge", ".article-row"],
		["Newspaper", ".masthead"],
	]) {
		button(target, label!, "nav button").click();
		await tick();
		const loading = target.querySelector('main [aria-busy="true"]');
		expect(loading?.querySelector("h1")?.textContent).toBe(label);
		expect(loading?.querySelector(shape!)).not.toBeNull();
	}
});

it("does not reactivate Dashboard when its listener resolves after navigation", async () => {
	setupCommands();
	const registration = deferred<() => void>();
	vi.mocked(listen).mockImplementation((event) =>
		event === "dashboard-source-updated" ? registration.promise : Promise.resolve(() => {}),
	);
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() => expect(button(target, "Memos", "nav button")).toBeTruthy());
	await tick();
	button(target, "Memos", "nav button").click();
	await tick();
	registration.resolve(() => {});
	await registration.promise;
	await tick();
	expect(invoke).not.toHaveBeenCalledWith("set_dashboard_active", { active: true });
	expect(invoke).not.toHaveBeenCalledWith("refresh_dashboard", expect.anything());
});

it("subscribes to ntfy only in Inbox and stops on leaving it", async () => {
	setupCommands();
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() => expect(button(target, "Memos", "nav button")).toBeTruthy());
	await tick();
	expect(invoke).not.toHaveBeenCalledWith("set_notifications_active", { active: true });
	const inbox = target.querySelector<HTMLButtonElement>('[aria-label="Open inbox"]');
	if (!inbox) throw new Error("Inbox action missing");
	inbox.click();
	await vi.waitFor(() =>
		expect(invoke).toHaveBeenCalledWith("set_notifications_active", { active: true }),
	);
	button(target, "Memos", "nav button").click();
	await vi.waitFor(() =>
		expect(invoke).toHaveBeenCalledWith("set_notifications_active", { active: false }),
	);
});

it("does not reactivate Dashboard when its listener resolves after unmount", async () => {
	setupCommands();
	const registration = deferred<() => void>();
	const unsubscribe = vi.fn();
	vi.mocked(listen).mockImplementation((event) =>
		event === "dashboard-source-updated" ? registration.promise : Promise.resolve(() => {}),
	);
	const target = document.createElement("div");
	document.body.append(target);
	const app = mount(App, { target });
	await tick();
	await unmount(app);
	registration.resolve(unsubscribe);
	await vi.waitFor(() => expect(unsubscribe).toHaveBeenCalled());
	expect(invoke).not.toHaveBeenCalledWith("set_dashboard_active", { active: true });
	expect(invoke).not.toHaveBeenCalledWith("refresh_dashboard", expect.anything());
});

it("starts a fresh Dashboard request after returning while the old Todo read is pending", async () => {
	setupCommands();
	const normal = invoke.getMockImplementation();
	if (!normal) throw new Error("Missing command mock");
	invoke.mockImplementation((command: string) => {
		if (command === "read_todos") return new Promise(() => {});
		return normal(command);
	});
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() =>
		expect(invoke).toHaveBeenCalledWith("refresh_dashboard", expect.anything()),
	);
	button(target, "Memos", "nav button").click();
	await tick();
	const before = invoke.mock.calls.filter(([command]) => command === "refresh_dashboard").length;
	button(target, "Dashboard", "nav button").click();
	await vi.waitFor(() =>
		expect(invoke.mock.calls.filter(([command]) => command === "refresh_dashboard")).toHaveLength(
			before + 1,
		),
	);
});

it("shows only configured destinations after configuration loads", async () => {
	setupCommands();
	const pending = deferred<CommandResponse<ConfigurationStatus>>();
	const normal = invoke.getMockImplementation();
	if (!normal) throw new Error("Missing command mock");
	invoke.mockImplementation((command: string) =>
		command === "read_configuration" ? pending.promise : normal(command),
	);
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await tick();
	const labels = () =>
		Array.from(target.querySelectorAll('nav[aria-label="Consumer views"] button'), (item) =>
			item.textContent?.trim(),
		);
	expect(labels()).toEqual(["Dashboard", "Settings"]);
	pending.resolve({
		status: "ready",
		data: {
			...configuration,
			api: {
				memos: { status: "missing" },
				moment: { status: "missing" },
				knowledge: { status: "ready", data: "test-knowledge" },
			},
			ntfy: { status: "missing" },
			qqMusic: { status: "ready", data: "Connected" },
		},
	});
	await vi.waitFor(() =>
		expect(labels()).toEqual(["Dashboard", "Newspaper", "Music", "Knowledge", "Settings"]),
	);
	expect(target.querySelector('[aria-label="Open inbox"]')).toBeNull();
	expect(target.querySelector("aside")?.textContent).not.toContain("Connections");
});

it("keeps one newspaper loading surface from index lookup through article compilation", async () => {
	setupCommands();
	const fallback = invoke.getMockImplementation()!;
	const index = deferred<CommandResponse<ChannelView>>();
	const detail = deferred<CommandResponse<import("../lib/consumer").KnowledgeDocument>>();
	invoke.mockImplementation((command: string, args: unknown) => {
		if (command === "read_channel") return index.promise;
		if (command === "read_knowledge") return detail.promise;
		return fallback(command, args);
	});
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await tick();
	await vi.waitFor(() => expect(button(target, "Newspaper", "nav button")).toBeDefined());
	button(target, "Newspaper", "nav button").click();
	await tick();
	const loading = target.querySelector('[aria-label="Loading newspaper"]');
	expect(loading).not.toBeNull();
	const entry = {
		id: "daily",
		slug: "daily",
		title: "Daily",
		summary: "Summary",
		tags: [],
		visibility: "private" as const,
		contentHash: "hash",
		createdAt: "2026-09-12",
		updatedAt: "2026-09-12",
		newspaperEdition: "developer" as const,
	};
	index.resolve({
		status: "ready",
		data: {
			channel: "knowledge",
			knowledge: [entry],
			newspaper: { developer: "daily", personal: null },
			nextCursor: null,
		},
	});
	await vi.waitFor(() =>
		expect(invoke).toHaveBeenCalledWith("read_knowledge", { id: "daily", expectedHash: "hash" }),
	);
	expect(target.querySelector('[aria-label="Loading newspaper"]')).toBe(loading);
	expect(target.textContent).not.toContain("Loading edition");
	expect(target.textContent).not.toContain("Preparing the article");
	expect(target.querySelectorAll('[aria-label="Loading newspaper"]')).toHaveLength(1);
	detail.resolve({
		status: "ready",
		data: {
			...entry,
			source: "Daily body",
			html: "<p>Daily body</p>",
			toc: [],
			stats: { wordCount: 1, readingMinutes: 1 },
		},
	});
	await vi.waitFor(() => expect(target.querySelector(".copy")?.textContent).toBe("Daily body"));
	expect(target.querySelector('[aria-label="Loading newspaper"]')).toBeNull();
});

it("starts a fresh SSH connection on each sidebar click, including after idle expiry", async () => {
	setupCommands();
	const fallback = invoke.getMockImplementation();
	if (fallback === undefined) throw new Error("Missing command setup");
	const launches: Array<{
		request: { sessionId: string };
		output: { onmessage: (message: SshOutput) => void };
	}> = [];
	invoke.mockImplementation((command: string, args) => {
		if (command === "read_ssh_devices")
			return Promise.resolve({
				status: "ready",
				data: {
					devices: [
						{
							id: "node",
							name: "NAS",
							address: "100.64.0.2",
							dnsName: "nas.test.ts.net",
							os: "linux",
							online: true,
							username: "admin",
						},
					],
					error: null,
				},
			});
		if (command === "connect_ssh") launches.push(args);
		if (["set_ssh_active", "connect_ssh", "disconnect_ssh"].includes(command))
			return Promise.resolve({ status: "ready", data: null });
		return fallback(command, args);
	});
	vi.stubGlobal(
		"ResizeObserver",
		class {
			observe() {}
			unobserve() {}
			disconnect() {}
		},
	);
	vi.stubGlobal("matchMedia", () => ({ matches: false }));
	const target = document.createElement("div");
	document.body.append(target);
	views.push(mount(App, { target }));
	await vi.waitFor(() => expect(target.querySelector('[aria-label^="NAS,"]')).not.toBeNull());
	const device = target.querySelector<HTMLButtonElement>('[aria-label^="NAS,"]');
	if (device === null) throw new Error("Missing SSH device");
	device.click();
	await vi.waitFor(() => expect(launches).toHaveLength(1));
	device.click();
	await vi.waitFor(() => expect(launches).toHaveLength(2));
	const [first, second] = launches;
	if (first === undefined || second === undefined) throw new Error("Missing SSH connections");
	expect(invoke).toHaveBeenCalledWith("disconnect_ssh", {
		sessionId: first.request.sessionId,
	});
	expect(second.request.sessionId).not.toBe(first.request.sessionId);
	second.output.onmessage({
		kind: "error",
		message: "Disconnected after 5 minutes without terminal activity.",
	});
	await tick();
	expect(target.textContent).toContain("Disconnected after 5 minutes");
	button(target, "Dashboard", "nav button").click();
	await tick();
	device.click();
	await vi.waitFor(() => expect(launches).toHaveLength(3));
	expect(target.textContent).not.toContain("Disconnected after 5 minutes");
	expect(target.querySelectorAll(".ssh-terminal")).toHaveLength(1);
	first.output.onmessage({ kind: "error", message: "Late old-session failure" });
	await tick();
	expect(target.textContent).not.toContain("Late old-session failure");
});
