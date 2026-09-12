import { beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { listen } from "@tauri-apps/api/event";
import { fromStore, writable } from "svelte/store";
import type { CommandResponse, Habit, TodoList } from "../../../consumer";
import CalendarPanel from "../CalendarPanel.svelte";

const { invoke, events } = vi.hoisted(() => ({
	invoke: vi.fn(),
	events: new Map<string, (event: { payload: string }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn(async (name: string, callback: (event: { payload: string }) => void) => {
		events.set(name, callback);
		return () => {
			if (events.get(name) === callback) events.delete(name);
		};
	}),
}));
beforeEach(() => {
	invoke.mockReset().mockResolvedValue({ status: "ready", data: [] });
	events.clear();
});

it.each([
	["2026-02-01", 0, 28],
	["2026-08-01", 6, 31],
	["2024-02-01", 4, 29],
	["0001-01-01", 1, 31],
	["0096-02-01", 3, 29],
])(
	"places %s in a Sunday-first month with the correct number of days",
	async (date, offset, days) => {
		const target = document.createElement("div");
		const onselect = vi.fn().mockResolvedValue(undefined);
		const view = mount(CalendarPanel, {
			target,
			props: { todayDate: date, selectedDate: date, onselect },
		});
		await tick();
		expect(Array.from(target.querySelectorAll(".weekday"), (day) => day.textContent)).toEqual([
			"S",
			"M",
			"T",
			"W",
			"T",
			"F",
			"S",
		]);
		expect(target.querySelectorAll(".calendar-spacer")).toHaveLength(offset);
		expect(target.querySelectorAll(".month-calendar button")).toHaveLength(days);
		expect(target.querySelector('[aria-pressed="true"]')?.textContent).toBe("1");
		await unmount(view);
	},
);

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (!element) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("selects a date across the year boundary", async () => {
	const target = document.createElement("div");
	const onselect = vi.fn().mockResolvedValue(undefined);
	const view = mount(CalendarPanel, {
		target,
		props: { todayDate: "2026-12-31", selectedDate: "2026-12-31", onselect },
	});
	try {
		await tick();
		findElement<HTMLButtonElement>(target, '[aria-label="Next month"]').click();
		await tick();
		findElement<HTMLButtonElement>(target, '[aria-label="Select 2027-01-01"]').click();
		expect(onselect).toHaveBeenCalledExactlyOnceWith("2027-01-01");
	} finally {
		await unmount(view);
	}
});

it.each([
	["0001-01-01", "Previous month"],
	["9999-12-31", "Next month"],
])("keeps %s within the supported calendar range", async (date, action) => {
	const target = document.createElement("div");
	const view = mount(CalendarPanel, {
		target,
		props: { todayDate: date, selectedDate: date, onselect: vi.fn() },
	});
	try {
		await tick();
		expect(findElement<HTMLButtonElement>(target, `[aria-label="${action}"]`).disabled).toBe(true);
		expect(
			target.querySelector(`[aria-label="Select ${date}"]`)?.getAttribute("aria-pressed"),
		).toBe("true");
	} finally {
		await unmount(view);
	}
});

it("discards late completion reads after navigating months", async () => {
	let settle: (value: CommandResponse<string[]>) => void = () => {};
	invoke.mockImplementation((_command: string, { date }: { date: string }) =>
		date === "2026-09-01"
			? new Promise<CommandResponse<string[]>>((resolve) => {
					settle = resolve;
				})
			: Promise.resolve({ status: "ready", data: ["2026-08-12"] }),
	);
	const target = document.createElement("div");
	const view = mount(CalendarPanel, {
		target,
		props: {
			todayDate: "2026-09-12",
			selectedDate: "2026-09-12",
			habits: [{ id: "read", name: "Read" }],
			onselect: vi.fn(),
		},
	});
	try {
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("read_planner_days", {
				ids: ["read"],
				date: "2026-09-01",
			}),
		);
		findElement<HTMLButtonElement>(target, '[aria-label="Previous month"]').click();
		await vi.waitFor(() =>
			expect(target.querySelector('[aria-label="Select 2026-08-12"] .completion')).not.toBeNull(),
		);
		settle({ status: "ready", data: ["2026-09-12"] });
		await tick();
		await tick();
		expect(target.querySelector('[aria-label="Select 2026-08-12"] .completion')).not.toBeNull();
	} finally {
		await unmount(view);
	}
});

it("invalidates marks on mutations and recovers after a failed read", async () => {
	invoke.mockResolvedValue({ status: "ready", data: ["2026-09-12"] });
	const target = document.createElement("div");
	const view = mount(CalendarPanel, {
		target,
		props: {
			todayDate: "2026-09-12",
			selectedDate: "2026-09-12",
			habits: [{ id: "read", name: "Read" }],
			onselect: vi.fn(),
		},
	});
	try {
		await vi.waitFor(() => expect(target.querySelector(".completion")).not.toBeNull());
		expect(
			target.querySelector('[aria-label="Select 2026-09-12"]')?.getAttribute("aria-describedby"),
		).toBe(target.querySelector(".sr-only")?.id);
		invoke.mockResolvedValue({ status: "failed", message: "Read failed" });
		events.get("check-in-updated")?.({ payload: "read" });
		await vi.waitFor(() => expect(target.textContent).toContain("Read failed"));
		expect(target.querySelector(".completion")).toBeNull();
		invoke.mockResolvedValue({ status: "ready", data: ["2026-09-12"] });
		window.dispatchEvent(new Event("focus"));
		await vi.waitFor(() => expect(target.querySelector(".completion")).not.toBeNull());
		invoke.mockResolvedValue({ status: "ready", data: [] });
		events.get("todo-updated")?.({ payload: "2026-09-12" });
		await vi.waitFor(() => expect(target.querySelector(".completion")).toBeNull());
	} finally {
		await unmount(view);
	}
});

it("rereads completion when configured habits change", async () => {
	invoke.mockResolvedValue({ status: "ready", data: ["2026-09-12"] });
	const habits = writable<Habit[]>([]);
	const state = fromStore(habits);
	const target = document.createElement("div");
	const view = mount(CalendarPanel, {
		target,
		props: {
			todayDate: "2026-09-12",
			selectedDate: "2026-09-12",
			get habits() {
				return state.current;
			},
			onselect: vi.fn(),
		},
	});
	try {
		await vi.waitFor(() => expect(target.querySelector(".completion")).not.toBeNull());
		invoke.mockResolvedValue({ status: "ready", data: [] });
		habits.set([{ id: "walk", name: "Walk" }]);
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenLastCalledWith("read_planner_days", {
				ids: ["walk"],
				date: "2026-09-01",
			}),
		);
		expect(target.querySelector(".completion")).toBeNull();
	} finally {
		await unmount(view);
	}
});

it("refreshes after a local todo projection changes", async () => {
	invoke.mockResolvedValue({ status: "ready", data: ["2026-09-12"] });
	const todos = writable<TodoList | null>(null);
	const state = fromStore(todos);
	const target = document.createElement("div");
	const view = mount(CalendarPanel, {
		target,
		props: {
			todayDate: "2026-09-12",
			selectedDate: "2026-09-12",
			get todos() {
				return state.current;
			},
			onselect: vi.fn(),
		},
	});
	try {
		await vi.waitFor(() => expect(target.querySelector(".completion")).not.toBeNull());
		invoke.mockResolvedValue({ status: "ready", data: [] });
		todos.set({ date: "2026-09-12", items: [], syncError: null });
		await vi.waitFor(() => expect(target.querySelector(".completion")).toBeNull());
		expect(invoke).toHaveBeenCalledTimes(2);
	} finally {
		await unmount(view);
	}
});

it("reads and refreshes when a live listener fails, and cleans up successful listeners", async () => {
	vi.mocked(listen).mockRejectedValueOnce(new Error("Listener unavailable"));
	invoke.mockResolvedValue({ status: "ready", data: ["2026-09-12"] });
	const target = document.createElement("div");
	const view = mount(CalendarPanel, {
		target,
		props: { todayDate: "2026-09-12", selectedDate: "2026-09-12", onselect: vi.fn() },
	});
	try {
		await vi.waitFor(() => expect(target.querySelector(".completion")).not.toBeNull());
		expect(target.textContent).toContain("Live calendar updates are unavailable");
		expect(events.size).toBe(2);
		invoke.mockResolvedValue({ status: "ready", data: [] });
		window.dispatchEvent(new Event("focus"));
		await vi.waitFor(() => expect(target.querySelector(".completion")).toBeNull());
		expect(target.textContent).toContain("Live calendar updates are unavailable");
	} finally {
		await unmount(view);
	}
	await vi.waitFor(() => expect(events.size).toBe(0));
});
