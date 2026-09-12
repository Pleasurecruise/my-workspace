import { expect, it, vi } from "vite-plus/test";
import { mount, unmount } from "svelte";
import type { CheckIn, CommandResponse, ExpenseSnapshot, TodoList } from "../../../consumer";
import { createDashboardSession } from "../session.svelte";
import WidgetContent from "../WidgetContent.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("svelte", async (original) => ({
	...(await original<typeof import("svelte")>()),
	onMount: vi.fn(),
}));

it("links calendar, tasks and check-ins while a previous task read is still pending", async () => {
	const first: TodoList = { date: "2024-02-28", items: [], syncError: null };
	const next: TodoList = {
		date: "2024-02-29",
		items: [
			{
				id: "walk",
				text: "Leap day walk",
				completed: false,
				rollover: false,
				description: null,
				details: null,
			},
		],
		syncError: null,
	};
	const state: CheckIn = {
		id: "read",
		editable: true,
		date: next.date,
		completed: false,
		streak: 0,
		total: 0,
		days: [],
	};
	let finish: (response: CommandResponse<TodoList>) => void = () => {
		throw new Error("Not initialized");
	};
	const pending = new Promise<CommandResponse<TodoList>>((resolve) => {
		finish = resolve;
	});
	invoke.mockResolvedValueOnce({ status: "ready", data: first });
	const session = createDashboardSession(() => false);
	session.selectDate(first.date);
	await session.loadTodos();
	invoke.mockImplementation(async (command: string, args: { date: string }) => {
		if (command === "read_todos")
			return args.date === first.date ? pending : { status: "ready", data: next };
		if (command === "read_check_ins")
			return { status: "ready", data: [{ ...state, date: args.date }] };
		if (command === "set_check_in") return { status: "ready", data: { ...state, completed: true } };
		throw new Error(`Unexpected command: ${command}`);
	});
	const target = document.createElement("div");
	const view = mount(WidgetContent, {
		target,
		props: {
			placement: {
				id: "planner",
				widget: { kind: "planner", habits: [{ id: "read", name: "Read" }] },
			},
			session,
			serviceCatalog: [],
		},
	});
	try {
		await vi.waitFor(() =>
			expect(target.querySelector(".habits .subtitle")?.textContent).toBe(first.date),
		);
		const previousRead = session.loadTodos();
		await vi.waitFor(() => expect(session.todos.loading).toBe(true));
		const day = target.querySelector<HTMLButtonElement>('[aria-label="Select 2024-02-29"]');
		expect(day?.disabled).toBe(false);
		day?.click();
		await vi.waitFor(() => expect(target.textContent).toContain("Leap day walk"));
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("read_check_ins", { ids: ["read"], date: next.date }),
		);
		expect(target.querySelector(".habits .subtitle")?.textContent).toBe(next.date);
		await vi.waitFor(() =>
			expect(
				target.querySelector<HTMLButtonElement>('[aria-label="Check in Read"]')?.disabled,
			).toBe(false),
		);
		target.querySelector<HTMLButtonElement>('[aria-label="Check in Read"]')?.click();
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("set_check_in", {
				id: "read",
				date: next.date,
				completed: true,
			}),
		);
		finish({ status: "ready", data: first });
		await previousRead;
		expect(session.selectedDate).toBe(next.date);
		expect(session.todos.data?.date).toBe(next.date);
	} finally {
		finish({ status: "ready", data: first });
		await unmount(view);
	}
});

it("shares Spending day navigation with Planner", async () => {
	const initial = "2026-09-11";
	const next = "2026-09-10";
	invoke.mockImplementation(async (command: string, args: { date: string }) => {
		if (command === "read_todos")
			return { status: "ready", data: { date: args.date, items: [], syncError: null } };
		if (command === "read_check_ins") return { status: "ready", data: [] };
		if (command === "read_expenses") {
			const data: ExpenseSnapshot = {
				date: args.date,
				month: args.date.slice(0, 7),
				entries: [],
				dayTotalPence: 0,
				monthTotalPence: 0,
				days: [],
				categories: [],
				suggestions: [],
			};
			return { status: "ready", data };
		}
		throw new Error(`Unexpected command: ${command}`);
	});
	const session = createDashboardSession(() => false);
	session.selectDate(initial);
	await session.loadTodos();
	const plannerTarget = document.createElement("div");
	const spendingTarget = document.createElement("div");
	const planner = mount(WidgetContent, {
		target: plannerTarget,
		props: {
			placement: { id: "planner", widget: { kind: "planner", habits: [] } },
			session,
			serviceCatalog: [],
		},
	});
	const spending = mount(WidgetContent, {
		target: spendingTarget,
		props: {
			placement: { id: "spending", widget: { kind: "spending" } },
			session,
			serviceCatalog: [],
		},
	});
	try {
		await vi.waitFor(() => expect(plannerTarget.querySelector(".month-calendar")).not.toBeNull());
		spendingTarget.querySelector<HTMLButtonElement>('[aria-label="Previous day"]')?.click();
		await vi.waitFor(() =>
			expect(
				plannerTarget.querySelector(`[aria-label="Select ${next}"]`)?.getAttribute("aria-pressed"),
			).toBe("true"),
		);
		expect(session.selectedDate).toBe(next);
		expect(session.todos.data?.date).toBe(next);
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("read_check_ins", { ids: [], date: next }),
		);
		await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("read_expenses", { date: next }));
	} finally {
		await unmount(planner);
		await unmount(spending);
	}
});

it("loads Spending without Todo or a separate date picker", async () => {
	invoke.mockReset();
	invoke.mockImplementation(async (command: string, args: { date: string }) => {
		if (command !== "read_expenses") throw new Error(`Unexpected command: ${command}`);
		return {
			status: "ready",
			data: {
				date: args.date,
				month: args.date.slice(0, 7),
				entries: [],
				dayTotalPence: 0,
				monthTotalPence: 0,
				days: [],
				categories: [],
				suggestions: [],
			},
		};
	});
	const session = createDashboardSession(() => false);
	session.selectDate("2026-09-11");
	const target = document.createElement("div");
	const view = mount(WidgetContent, {
		target,
		props: {
			placement: { id: "spending", widget: { kind: "spending" } },
			session,
			serviceCatalog: [],
		},
	});
	try {
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("read_expenses", { date: "2026-09-11" }),
		);
		expect(target.querySelector('input[type="date"]')).toBeNull();
		expect(target.querySelector('[aria-label="Refresh expenses"]')).toBeNull();

		expect(invoke.mock.calls.every(([command]) => command === "read_expenses")).toBe(true);
	} finally {
		await unmount(view);
	}
});
