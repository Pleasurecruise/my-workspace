import { beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { fromStore, writable } from "svelte/store";
import type { CommandResponse, ExpenseSnapshot } from "../../../consumer";
import SpendingPanel from "../SpendingPanel.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
beforeEach(() => {
	invoke.mockReset();
});

const data: ExpenseSnapshot = {
	date: "2026-09-11",
	month: "2026-09",
	entries: [{ id: "lunch", date: "2026-09-11", amountPence: 1234, category: "Dining" }],
	dayTotalPence: 1234,
	monthTotalPence: 2000,
	categories: [
		{ category: "Dining", amountPence: 1234 },
		{ category: "Transport", amountPence: 766 },
	],
	days: Array.from({ length: 30 }, (_, index) => ({
		date: `2026-09-${String(index + 1).padStart(2, "0")}`,
		amountPence: index === 10 ? 1234 : index === 11 ? 766 : 0,
	})),
	suggestions: ["Dining", "Transport"],
};

async function input(target: HTMLElement, label: string, value: string) {
	if (label === "Expense category") {
		target.querySelector<HTMLButtonElement>('[role="combobox"]')?.click();
		await tick();
		Array.from(target.querySelectorAll<HTMLButtonElement>('[role="option"]'))
			.find((option) => option.textContent?.includes("Custom category"))
			?.click();
		await tick();
		label = "Custom expense category";
	}
	const element = target.querySelector<HTMLInputElement>(`[aria-label="${label}"]`);
	if (element === null) throw new Error(`Missing ${label}`);
	element.value = value;
	element.dispatchEvent(new Event("input", { bubbles: true }));
	return element;
}

it("renders monthly charts without a native picker or refresh button", async () => {
	const onselect = vi.fn();
	invoke.mockResolvedValue({ status: "ready", data });
	const target = document.createElement("div");
	const view = mount(SpendingPanel, {
		target,
		props: { selectedDate: data.date, todayDate: "2026-09-12", onselect },
	});
	try {
		await vi.waitFor(() =>
			expect(target.querySelector(".month-heading")?.textContent).toContain("£20.00"),
		);
		expect(target.querySelectorAll("circle")).toHaveLength(2);
		expect(target.querySelectorAll(".bar-chart .day")).toHaveLength(0);
		expect(target.querySelector(".legend")?.textContent).toContain("61.7%");
		expect(target.querySelector(".daily-total")?.textContent).toContain("£12.34");
		const calls = invoke.mock.calls.length;
		const toggle = Array.from(
			target.querySelectorAll<HTMLButtonElement>(".chart-switch button"),
		).find((button) => button.textContent === "Daily spending");
		toggle?.click();
		await tick();
		expect(target.querySelectorAll(".bar-chart .day")).toHaveLength(30);
		expect(target.querySelectorAll("circle")).toHaveLength(0);
		expect(invoke).toHaveBeenCalledTimes(calls);
		expect(target.querySelector('input[type="date"]')).toBeNull();
		expect(target.querySelector('[aria-label="Refresh expenses"]')).toBeNull();
		expect(target.querySelector(".bar-chart button")).toBeNull();
		expect(target.querySelector("time")?.textContent).toBe(data.date);
		target.querySelector<HTMLButtonElement>('[aria-label="Previous day"]')?.click();
		target.querySelector<HTMLButtonElement>('[aria-label="Next day"]')?.click();
		Array.from(target.querySelectorAll<HTMLButtonElement>(".date-controls button"))
			.find((button) => button.textContent === "Today")
			?.click();
		expect(onselect.mock.calls).toEqual([["2026-09-10"], ["2026-09-12"], ["2026-09-12"]]);
		expect(target.querySelector("datalist, select")).toBeNull();
		target.querySelector<HTMLButtonElement>('[role="combobox"]')?.click();
		await tick();
		Array.from(target.querySelectorAll<HTMLButtonElement>('[role="option"]'))
			.find((option) => option.textContent === "Dining")
			?.click();
		await tick();
		expect(target.querySelector('[role="combobox"]')?.textContent).toContain("Dining");
		expect(target.querySelector('[aria-label="Custom expense category"]')).toBeNull();
	} finally {
		await unmount(view);
	}
});

it("submits only amount and category for the selected day and preserves edits made during save", async () => {
	let finish: (value: CommandResponse<ExpenseSnapshot>) => void = () => {
		throw new Error("Not initialized");
	};
	const pending = new Promise<CommandResponse<ExpenseSnapshot>>((resolve) => {
		finish = resolve;
	});
	invoke.mockImplementation((command) =>
		command === "create_expense" ? pending : Promise.resolve({ status: "ready", data }),
	);
	const target = document.createElement("div");
	const view = mount(SpendingPanel, {
		target,
		props: { selectedDate: data.date, todayDate: data.date, onselect: () => {} },
	});
	try {
		await vi.waitFor(() => expect(target.querySelectorAll("circle")).toHaveLength(2));
		await input(target, "Expense amount in GBP", "4.56");
		await input(target, "Expense category", "Coffee");
		await tick();
		target.querySelector("form")?.dispatchEvent(new Event("submit", { cancelable: true }));
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("create_expense", {
				date: data.date,
				amount: "4.56",
				category: "Coffee",
			}),
		);
		expect(target.querySelector<HTMLButtonElement>('[type="submit"]')?.disabled).toBe(true);
		const field = await input(target, "Expense amount in GBP", "9.99");
		finish({ status: "ready", data });
		await vi.waitFor(() =>
			expect(target.querySelector<HTMLButtonElement>('[type="submit"]')?.disabled).toBe(false),
		);
		expect(field.value).toBe("9.99");
	} finally {
		finish({ status: "ready", data });
		await unmount(view);
	}
});

it("edits and deletes entries, updates the chart, and reports failed saves", async () => {
	const empty = {
		...data,
		entries: [],
		categories: [],
		dayTotalPence: 0,
		monthTotalPence: 0,
		days: data.days.map((day) => ({ ...day, amountPence: 0 })),
	};
	invoke.mockImplementation(async (command) =>
		command === "update_expense"
			? { status: "failed", message: "Invalid amount" }
			: { status: "ready", data: command === "delete_expense" ? empty : data },
	);
	const target = document.createElement("div");
	const view = mount(SpendingPanel, {
		target,
		props: { selectedDate: data.date, todayDate: data.date, onselect: () => {} },
	});
	try {
		await vi.waitFor(() => expect(target.querySelector('[title="Edit expense"]')).not.toBeNull());
		target.querySelector<HTMLButtonElement>('[title="Edit expense"]')?.click();
		await tick();
		expect(
			target.querySelector<HTMLInputElement>('[aria-label="Expense amount in GBP"]')?.value,
		).toBe("12.34");
		await input(target, "Expense amount in GBP", "12.345");
		target.querySelector("form")?.dispatchEvent(new Event("submit", { cancelable: true }));
		await vi.waitFor(() =>
			expect(target.querySelector('[role="alert"]')?.textContent).toBe("Invalid amount"),
		);
		expect(invoke).toHaveBeenCalledWith("update_expense", {
			date: data.date,
			id: "lunch",
			amount: "12.345",
			category: "Dining",
		});
		target.querySelector<HTMLButtonElement>('[title="Delete expense"]')?.click();
		await vi.waitFor(() => expect(target.textContent).toContain("No spending this month"));
		expect(target.querySelectorAll("circle")).toHaveLength(0);
		expect(target.querySelector(".daily-total")?.textContent).toContain("£0.00");
	} finally {
		await unmount(view);
	}
});

it("reloads the month and clears the previous projection when the shared date changes", async () => {
	const selection = writable(data.date);
	const date = fromStore(selection);
	invoke.mockImplementation(async (_command, args: { date: string }) => ({
		status: "ready",
		data: { ...data, date: args.date, month: args.date.slice(0, 7) },
	}));
	const target = document.createElement("div");
	const view = mount(SpendingPanel, {
		target,
		props: {
			todayDate: data.date,
			onselect: () => {},
			get selectedDate() {
				return date.current;
			},
		},
	});
	try {
		await vi.waitFor(() =>
			expect(target.querySelector(".month-heading")?.textContent).toContain("September 2026"),
		);
		selection.set("2026-10-01");
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenLastCalledWith("read_expenses", { date: "2026-10-01" }),
		);
		expect(target.querySelector(".month-heading")?.textContent).toContain("October 2026");
	} finally {
		await unmount(view);
	}
});
