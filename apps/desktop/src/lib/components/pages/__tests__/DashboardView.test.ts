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
beforeEach(() => invoke.mockReset());

it("adds named check-ins from Personal, rejects duplicates, and retains the form after a failed save", async () => {
	const initial: WidgetLayout = {
		widgets: [{ id: "check-in-read", widget: { kind: "checkIn", name: "Read" } }],
		islandWidgetId: "check-in-read",
	};
	invoke.mockResolvedValueOnce({ status: "ready", data: initial });
	const layoutSession = createLayoutSession();
	await layoutSession.load();
	const session = createDashboardSession(() => false);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(DashboardView, { target, props: { session, layoutSession } });
	function click(selector: string) {
		const button = target.querySelector<HTMLButtonElement>(selector);
		if (button === null) throw new Error(`Missing ${selector}`);
		button.click();
	}
	try {
		await tick();
		click('button[aria-label="Edit dashboard"]');
		await tick();
		click(".add-widget-button");
		await tick();
		const personal = Array.from(target.querySelectorAll("button")).find(
			(button) => button.textContent?.trim() === "Personal",
		);
		if (personal === undefined) throw new Error("Personal category is missing");
		personal.click();
		await tick();
		expect(target.textContent).toContain("Check-in");
		const input = target.querySelector<HTMLInputElement>(
			'input[placeholder="For example: Read for 20 minutes"]',
		);
		if (input === null) throw new Error("Check-in name input is missing");
		click(".library-footer .primary-button");
		await tick();
		expect(target.textContent).toContain("Enter something you want to do each day");
		input.value = "read";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		click(".library-footer .primary-button");
		await tick();
		expect(target.textContent).toContain("This check-in is already on the Dashboard");
		expect(invoke).toHaveBeenCalledTimes(1);
		input.value = "  Walk  ";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		invoke.mockResolvedValueOnce({ status: "failed", message: "Storage unavailable" });
		click(".library-footer .primary-button");
		await tick();
		await tick();
		expect(input.value).toBe("  Walk  ");
		expect(target.textContent).toContain("Storage unavailable");
		expect(layoutSession.layout?.widgets).toHaveLength(1);
		invoke.mockResolvedValueOnce({ status: "ready", data: null });
		click(".library-footer .primary-button");
		await tick();
		await tick();
		expect(layoutSession.layout?.islandWidgetId).toBe("check-in-read");
		expect(layoutSession.layout?.widgets).toHaveLength(2);
		expect(layoutSession.layout?.widgets[1]).toEqual({
			id: expect.stringMatching(/^check-in-/),
			widget: { kind: "checkIn", name: "Walk" },
		});
		await vi.waitFor(() => expect(target.querySelector(".library-footer")).toBeNull());
	} finally {
		await unmount(view);
		target.remove();
	}
});
