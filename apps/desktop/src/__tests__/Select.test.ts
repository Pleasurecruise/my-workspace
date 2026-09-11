import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { fromStore, writable } from "svelte/store";
import { Select } from "@my-workspace/ui";

it("keeps keyboard options visible and closes when disabled", async () => {
	const state = fromStore(writable(false));
	const onchange = vi.fn();
	const scroll = vi.spyOn(HTMLElement.prototype, "scrollIntoView");
	const target = document.createElement("div");
	const view = mount(Select, {
		target,
		props: {
			label: "Account",
			value: "0",
			options: Array.from({ length: 30 }, (_, index) => ({
				value: String(index),
				label: `Account ${index}`,
			})),
			get disabled() {
				return state.current;
			},
			onchange,
		},
	});
	try {
		const trigger = target.querySelector("button");
		if (!trigger) throw new Error("Missing select trigger");
		trigger.click();
		await tick();
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
		await tick();
		expect(target.querySelector(".focused")?.textContent).toContain("Account 29");
		expect(scroll).toHaveBeenLastCalledWith({ block: "nearest" });
		expect(scroll.mock.contexts.at(-1)).toBe(target.querySelector(".focused"));
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
		await tick();
		expect(onchange).toHaveBeenCalledWith("29");
		trigger.click();
		await tick();
		state.current = true;
		await tick();
		expect(trigger.getAttribute("aria-expanded")).toBe("false");
		expect(target.querySelector('[role="listbox"]')).toBeNull();
	} finally {
		scroll.mockRestore();
		await unmount(view);
	}
});

it("never opens an empty list or references a nonexistent active option", async () => {
	const target = document.createElement("div");
	const view = mount(Select, {
		target,
		props: { label: "Account", value: "", options: [], onchange: vi.fn() },
	});
	try {
		const trigger = target.querySelector("button");
		if (!trigger) throw new Error("Missing select trigger");
		for (const key of ["ArrowDown", "ArrowUp", "Home", "End"]) {
			trigger.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
			await tick();
			expect(trigger.getAttribute("aria-expanded")).toBe("false");
			expect(trigger.getAttribute("aria-activedescendant")).toBe("");
		}
	} finally {
		await unmount(view);
	}
});
