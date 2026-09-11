import { afterEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, QqLoginStatus, QqQr } from "../../../consumer";
import QqMusicConnection from "../QqMusicConnection.svelte";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
afterEach(() => {
	vi.useRealTimers();
	invoke.mockReset();
});

it.each(["complete", "failed", "scanned"])(
	"ignores an old %s poll after reopening the QR",
	async (status) => {
		vi.useFakeTimers();
		let finish: (response: CommandResponse<QqLoginStatus>) => void = () => {
			throw new Error("Poll not started");
		};
		const pending = new Promise<CommandResponse<QqLoginStatus>>((resolve) => {
			finish = resolve;
		});
		let beginCount = 0;
		invoke.mockImplementation((command: string) => {
			if (command === "begin_qq_music_login")
				return Promise.resolve({ status: "ready", data: { image: `qr-${++beginCount}` } });
			if (command === "poll_qq_music_login") return pending;
			return Promise.resolve({ status: "ready", data: null });
		});
		const onconnected = vi.fn();
		const target = document.createElement("div");
		const view = mount(QqMusicConnection, { target, props: { connected: false, onconnected } });
		try {
			target.querySelector<HTMLButtonElement>(".settings-save-button")?.click();
			await tick();
			await vi.advanceTimersByTimeAsync(0);
			await vi.advanceTimersByTimeAsync(1500);
			expect(invoke).toHaveBeenCalledWith("poll_qq_music_login");
			target.querySelector<HTMLButtonElement>('[aria-label="Close QQ Music login"]')?.click();
			await tick();
			await vi.advanceTimersByTimeAsync(0);
			target.querySelector<HTMLButtonElement>(".settings-save-button")?.click();
			await tick();
			await vi.advanceTimersByTimeAsync(0);
			expect(target.querySelector(".qq-code img")?.getAttribute("src")).toBe("qr-2");
			finish(
				status === "failed"
					? { status: "failed", message: "Old login expired" }
					: { status: "ready", data: { status: status === "complete" ? "complete" : "scanned" } },
			);
			await tick();
			await vi.advanceTimersByTimeAsync(0);
			expect(target.querySelector(".qq-code img")?.getAttribute("src")).toBe("qr-2");
			expect(target.textContent).toContain("Waiting for scan");
			expect(target.textContent).not.toContain("Old login expired");
			expect(onconnected).not.toHaveBeenCalled();
		} finally {
			await unmount(view);
		}
	},
);

it("cancels a pending QR creation on unmount and never starts polling its late result", async () => {
	vi.useFakeTimers();
	let finish: (response: CommandResponse<QqQr>) => void = () => {
		throw new Error("Begin not started");
	};
	const pending = new Promise<CommandResponse<QqQr>>((resolve) => {
		finish = resolve;
	});
	invoke.mockImplementation((command: string) =>
		command === "begin_qq_music_login" ? pending : Promise.resolve({ status: "ready", data: null }),
	);
	const target = document.createElement("div");
	const onconnected = vi.fn();
	const view = mount(QqMusicConnection, { target, props: { connected: false, onconnected } });
	target.querySelector<HTMLButtonElement>(".settings-save-button")?.click();
	await tick();
	await vi.advanceTimersByTimeAsync(0);
	await unmount(view);
	expect(invoke).toHaveBeenCalledWith("cancel_qq_music_login");
	finish({ status: "ready", data: { image: "late-qr" } });
	await vi.advanceTimersByTimeAsync(5000);
	expect(invoke).not.toHaveBeenCalledWith("poll_qq_music_login");
	expect(onconnected).not.toHaveBeenCalled();
});
