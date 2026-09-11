import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import CalendarPanel from "../CalendarPanel.svelte";

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
