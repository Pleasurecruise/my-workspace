import { expect, it, vi } from "vite-plus/test";
import type { CommandResponse, ConfigurationStatus } from "../../../consumer";
import { createSettingsSession } from "../session.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

it.each<"ready" | "failed">(["ready", "failed"])(
	"ignores a stale %s configuration response",
	async (status) => {
		const current: ConfigurationStatus = {
			ugos: { status: "missing" },
			r2: { status: "missing" },
			api: {
				memos: { status: "missing" },
				moment: { status: "missing" },
				knowledge: { status: "missing" },
			},
			ntfy: { status: "missing" },
			ntfyDev: false,
			notionCalendar: { status: "missing" },
			appLock: { status: "missing" },
			appLockDev: false,
			spotify: { status: "missing" },
			qqMusic: { status: "ready", data: "Connected" },
			publication: { telegram: false, x: false },
		};
		let finish: (response: CommandResponse<ConfigurationStatus>) => void = () => {
			throw new Error("Read not started");
		};
		const pending = new Promise<CommandResponse<ConfigurationStatus>>((resolve) => {
			finish = resolve;
		});
		invoke.mockReset();
		invoke.mockReturnValueOnce(pending).mockResolvedValueOnce({ status: "ready", data: current });
		const session = createSettingsSession({
			resetChannel: vi.fn(),
			initializeConsumers: vi.fn(),
			refreshDashboard: vi.fn(),
		});
		const older = session.loadConfiguration();
		await session.loadConfiguration();
		finish(
			status === "ready"
				? { status: "ready", data: { ...current, qqMusic: { status: "missing" } } }
				: { status: "failed", message: "Old read failed" },
		);
		await older;
		expect(session.configuration).toEqual(current);
		expect(session.error).toBeNull();
	},
);
