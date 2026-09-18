import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import SshTerminal from "../SshTerminal.svelte";
import type { CommandResponse, SshOutput } from "../../../consumer";

type CommandArguments =
	| {
			request: {
				sessionId: string;
				deviceId: string;
				username: string;
				cols: number;
				rows: number;
			};
			output: { onmessage: (message: SshOutput) => void };
	  }
	| { sessionId: string; bytes: number[] | number }
	| { sessionId: string; cols: number; rows: number }
	| { sessionId: string };

const mocks = vi.hoisted(() => ({
	invoke: vi.fn<(name: string, args: CommandArguments) => Promise<CommandResponse<null>>>(),
	write: vi.fn<(data: Uint8Array | string, callback?: () => void) => void>(),
	dispose: vi.fn(),
	input: (_data: string): void => {
		throw new Error("Terminal input is not registered");
	},
}));
vi.mock("@tauri-apps/api/core", () => ({
	invoke: mocks.invoke,
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
		write = mocks.write;
		dispose = mocks.dispose;
		onData(callback: (data: string) => void) {
			mocks.input = callback;
		}
		onBinary() {}
	},
}));
vi.mock("@xterm/addon-fit", () => ({
	FitAddon: class {
		fit() {}
	},
}));
const views: ReturnType<typeof mount>[] = [];
beforeEach(() => {
	mocks.invoke.mockReset().mockResolvedValue({ status: "ready", data: null });
	mocks.write.mockReset().mockImplementation((_data, callback) => callback?.());
	mocks.dispose.mockClear();
	vi.stubGlobal(
		"ResizeObserver",
		class {
			observe() {}
			disconnect() {}
		},
	);
	vi.stubGlobal("matchMedia", () => ({ matches: false }));
});
afterEach(async () => {
	for (const view of views.splice(0)) await unmount(view);
	document.body.replaceChildren();
	vi.unstubAllGlobals();
});
function deferResponse() {
	let resolve = (_response: CommandResponse<null>): void => {
		throw new Error("Response promise is not initialized");
	};
	let reject = (_error: Error): void => {
		throw new Error("Response promise is not initialized");
	};
	const promise = new Promise<CommandResponse<null>>((settle, fail) => {
		resolve = settle;
		reject = fail;
	});
	return { promise, resolve, reject };
}
function readConnection() {
	const call = mocks.invoke.mock.calls.find(([name]) => name === "connect_ssh");
	if (call === undefined || !("request" in call[1]))
		throw new Error("Expected SSH connection request");
	return call[1];
}
function setup() {
	const view = mount(SshTerminal, {
		target: document.body,
		props: {
			device: {
				id: "node",
				name: "NAS",
				address: "100.64.0.2",
				dnsName: "nas.test.ts.net",
				os: "linux",
				online: true,
				username: "admin",
			},
			active: true,
			locked: false,
		},
	});
	views.push(view);
	return view;
}
it("transports typed terminal input and acknowledges rendered output, then closes on unmount", async () => {
	const view = setup();
	await vi.waitFor(() =>
		expect(mocks.invoke).toHaveBeenCalledWith("connect_ssh", expect.anything()),
	);
	const request = readConnection();
	request.output.onmessage({ kind: "data", bytes: [104, 105] });
	expect(mocks.write).toHaveBeenCalledWith(new Uint8Array([104, 105]), expect.any(Function));
	expect(mocks.invoke).toHaveBeenCalledWith("acknowledge_ssh", {
		sessionId: request.request.sessionId,
		bytes: 2,
	});
	mocks.input("ls\r");
	await vi.waitFor(() =>
		expect(mocks.invoke).toHaveBeenCalledWith("write_ssh", {
			sessionId: request.request.sessionId,
			bytes: [108, 115, 13],
		}),
	);
	await unmount(view);
	views.splice(views.indexOf(view), 1);
	expect(mocks.invoke).toHaveBeenCalledWith("disconnect_ssh", {
		sessionId: request.request.sessionId,
	});
	expect(mocks.dispose).toHaveBeenCalledOnce();
});
it("keeps a fast-exiting SSH session closed when the launch response arrives later", async () => {
	const pending = deferResponse();
	mocks.invoke.mockImplementation((name) =>
		name === "connect_ssh" ? pending.promise : Promise.resolve({ status: "ready", data: null }),
	);
	setup();
	await vi.waitFor(() =>
		expect(mocks.invoke).toHaveBeenCalledWith("connect_ssh", expect.anything()),
	);
	const request = readConnection();
	request.output.onmessage({ kind: "exit", code: 255 });
	pending.resolve({ status: "ready", data: null });
	await tick();
	await vi.waitFor(() =>
		expect(document.querySelector('[role="status"]')?.textContent).toBe("Disconnected"),
	);
	expect(document.body.textContent).toContain("Reconnect");
});

it("records user keyboard activity separately from automatic terminal replies", async () => {
	setup();
	await vi.waitFor(() =>
		expect(mocks.invoke).toHaveBeenCalledWith("connect_ssh", expect.anything()),
	);
	const request = readConnection();
	mocks.input("\x1b[1;1R");
	await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("write_ssh", expect.anything()));
	expect(mocks.invoke.mock.calls.some(([name]) => name === "record_ssh_activity")).toBe(false);
	const host = document.querySelector(".terminal-host");
	if (host === null) throw new Error("Expected terminal host");
	host.dispatchEvent(new KeyboardEvent("keydown", { key: "a", bubbles: true }));
	expect(mocks.invoke.mock.calls.some(([name]) => name === "record_ssh_activity")).toBe(false);
	const key = new KeyboardEvent("keydown", { key: "a", bubbles: true });
	Object.defineProperty(key, "isTrusted", { value: true });
	host.dispatchEvent(key);
	const wheel = new WheelEvent("wheel", { deltaY: 10, bubbles: true });
	Object.defineProperty(wheel, "isTrusted", { value: true });
	host.dispatchEvent(wheel);
	expect(mocks.invoke.mock.calls.filter(([name]) => name === "record_ssh_activity")).toHaveLength(
		2,
	);
	expect(mocks.invoke).toHaveBeenCalledWith("record_ssh_activity", {
		sessionId: request.request.sessionId,
	});
});

it("preserves the idle timeout reason when pending input fails before exit", async () => {
	const pending = deferResponse();
	mocks.invoke.mockImplementation((name) =>
		name === "write_ssh" ? pending.promise : Promise.resolve({ status: "ready", data: null }),
	);
	setup();
	await vi.waitFor(() =>
		expect(mocks.invoke).toHaveBeenCalledWith("connect_ssh", expect.anything()),
	);
	const request = readConnection();
	mocks.input("input");
	await vi.waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("write_ssh", expect.anything()));
	const reason = "Disconnected after 5 minutes without terminal activity.";
	request.output.onmessage({ kind: "error", message: reason });
	pending.resolve({ status: "failed", message: "This SSH session is closed." });
	await tick();
	await vi.waitFor(() =>
		expect(document.querySelector('[role="alert"]')?.textContent).toBe(reason),
	);
	expect(document.querySelector('[role="status"]')?.textContent).toBe("Disconnected");
	request.output.onmessage({ kind: "exit", code: 1 });
	await tick();
	expect(document.querySelector('[role="alert"]')?.textContent).toBe(reason);
});

it("preserves a terminal failure reason when the pending launch transport rejects", async () => {
	const pending = deferResponse();
	mocks.invoke.mockImplementation((name) =>
		name === "connect_ssh" ? pending.promise : Promise.resolve({ status: "ready", data: null }),
	);
	setup();
	await vi.waitFor(() =>
		expect(mocks.invoke).toHaveBeenCalledWith("connect_ssh", expect.anything()),
	);
	const request = readConnection();
	const reason = "The SSH terminal stopped accepting input.";
	request.output.onmessage({ kind: "error", message: reason });
	pending.reject(new Error("Late IPC failure"));
	await tick();
	await tick();
	expect(document.querySelector('[role="alert"]')?.textContent).toBe(reason);
});

it("ignores a previous disconnect failure after a new connection starts", async () => {
	const pending = deferResponse();
	mocks.invoke.mockImplementation((name) =>
		name === "disconnect_ssh" ? pending.promise : Promise.resolve({ status: "ready", data: null }),
	);
	setup();
	await vi.waitFor(() =>
		expect(document.querySelector('[role="status"]')?.textContent).toBe("SSH running"),
	);
	const disconnect = [...document.querySelectorAll("button")].find((button) =>
		button.textContent?.includes("Disconnect"),
	);
	if (disconnect === undefined) throw new Error("Expected disconnect button");
	disconnect.click();
	await tick();
	const form = document.querySelector("form");
	if (form === null) throw new Error("Expected connection form");
	form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
	await vi.waitFor(() =>
		expect(mocks.invoke.mock.calls.filter(([name]) => name === "connect_ssh")).toHaveLength(2),
	);
	pending.reject(new Error("Previous disconnect failed"));
	await tick();
	await tick();
	expect(document.querySelector('[role="alert"]')).toBeNull();
	expect(document.querySelector('[role="status"]')?.textContent).toBe("SSH running");
});
