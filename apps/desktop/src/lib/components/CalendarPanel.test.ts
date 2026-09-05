import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import CalendarPanel from "./CalendarPanel.svelte";

it.each([
  ["2026-02-01", 0, 28],
  ["2026-08-01", 6, 31],
  ["2024-02-01", 4, 29],
])(
  "places %s in a Sunday-first month with the correct number of days",
  async (date, offset, days) => {
    const target = document.createElement("div");
    const onselect = vi.fn().mockResolvedValue(undefined);
    const view = mount(CalendarPanel, {
      target,
      props: { todayDate: date, selectedDate: date, loading: false, onselect },
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

it("selects a date across the year boundary and prevents selection while loading", async () => {
  const target = document.createElement("div");
  const onselect = vi.fn().mockResolvedValue(undefined);
  let view = mount(CalendarPanel, {
    target,
    props: { todayDate: "2026-12-31", selectedDate: "2026-12-31", loading: false, onselect },
  });
  await tick();
  target.querySelector<HTMLButtonElement>('[aria-label="Next month"]')!.click();
  await tick();
  target.querySelector<HTMLButtonElement>('[aria-label="Select 2027-01-01"]')!.click();
  expect(onselect).toHaveBeenCalledExactlyOnceWith("2027-01-01");
  await unmount(view);
  view = mount(CalendarPanel, {
    target,
    props: { todayDate: "2026-12-31", selectedDate: "2027-01-01", loading: true, onselect },
  });
  await tick();
  const day = target.querySelector<HTMLButtonElement>('[aria-label="Select 2027-01-02"]')!;
  expect(day.disabled).toBe(true);
  day.click();
  expect(onselect).toHaveBeenCalledTimes(1);
  await unmount(view);
});
