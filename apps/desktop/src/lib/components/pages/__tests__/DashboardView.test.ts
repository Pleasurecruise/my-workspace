import { beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { WidgetLayout } from "../../../consumer";
import { createDashboardSession } from "../../dashboard/session.svelte";
import { createLayoutSession } from "../../dashboard/layout.svelte";
import DashboardView from "../DashboardView.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("svelte", async (original) => ({
	...(await original<typeof import("svelte")>()),
	onMount: vi.fn(),
}));
// These tests exercise library configuration and persistence; each card has its own tests.
vi.mock("../../dashboard/WidgetContent.svelte", () => ({ default: () => {} }));
beforeEach(() => {
	invoke.mockReset();
});

it("groups planner and spending under Personal", async () => {
	const initial: WidgetLayout = { widgets: [], islandWidgetId: null };
	invoke.mockResolvedValueOnce({ status: "ready", data: initial });
	const layoutSession = createLayoutSession();
	await layoutSession.load();
	const session = createDashboardSession(() => false);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(DashboardView, { target, props: { session, layoutSession } });
	try {
		await tick();
		target.querySelector<HTMLButtonElement>('button[aria-label="Edit dashboard"]')?.click();
		await tick();
		target.querySelector<HTMLButtonElement>(".add-widget-button")?.click();
		await tick();
		expect(
			Array.from(target.querySelectorAll(".category-list button"), (button) => button.textContent),
		).toEqual(["Personal", "Devices", "AI Services", "Online Services", "Games"]);
		expect(target.querySelector(".widget-list")?.textContent).toContain("Spending");
		expect(target.querySelector(".widget-list")?.textContent).not.toContain("CPU");
		const planner = Array.from(
			target.querySelectorAll<HTMLButtonElement>(".widget-list button"),
		).find((button) => button.textContent?.includes("Daily Planner"));
		if (!planner) throw new Error("Planner is missing");
		planner.click();
		await tick();
		invoke.mockResolvedValueOnce({ status: "ready", data: null });
		target.querySelector<HTMLButtonElement>(".library-footer .primary-button")?.click();
		await tick();
		await tick();
		expect(layoutSession.layout?.widgets).toEqual([
			{ id: "planner", widget: { kind: "planner", habits: [] } },
		]);
	} finally {
		await unmount(view);
		target.remove();
	}
});
