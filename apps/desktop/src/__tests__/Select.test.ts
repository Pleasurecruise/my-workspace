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

it("supports typeahead and commits an open-list selection only on Enter", async () => {
	const selected = fromStore(writable(""));
	const target = document.createElement("div");
	document.body.append(target);
	const onchange = vi.fn((value: string) => {
		selected.current = value;
	});
	const view = mount(Select, {
		target,
		props: {
			label: "Fruit",
			get value() {
				return selected.current;
			},
			onchange,
			options: [
				{ value: "apple", label: "Apple" },
				{ value: "apricot", label: "Apricot" },
				{ value: "banana", label: "Banana" },
			],
		},
	});
	try {
		await tick();
		const trigger = target.querySelector("button");
		if (!trigger) throw new Error("Missing select trigger");
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "a", bubbles: true }));
		await tick();
		expect(selected.current).toBe("apple");
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "A", bubbles: true }));
		await tick();
		expect(selected.current).toBe("apricot");
		trigger.click();
		await tick();
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "b", bubbles: true }));
		await tick();
		expect(target.querySelector(".focused")?.textContent).toContain("Banana");
		expect(selected.current).toBe("apricot");
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
		await tick();
		expect(selected.current).toBe("banana");
		expect(trigger.getAttribute("aria-expanded")).toBe("false");
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("matches a multi-character prefix and resets it after a pause", async () => {
	const selected = fromStore(writable(""));
	const target = document.createElement("div");
	const clock = vi.spyOn(Date, "now").mockReturnValue(1000);
	const view = mount(Select, {
		target,
		props: {
			label: "Fruit",
			get value() {
				return selected.current;
			},
			onchange: (value: string) => {
				selected.current = value;
			},
			options: [
				{ value: "apple", label: "Apple" },
				{ value: "apricot", label: "Apricot" },
				{ value: "banana", label: "Banana" },
			],
		},
	});
	try {
		await tick();
		const trigger = target.querySelector("button");
		if (!trigger) throw new Error("Missing select trigger");
		for (const key of ["a", "p", "r"]) {
			trigger.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
			await tick();
		}
		expect(selected.current).toBe("apricot");
		clock.mockReturnValue(1800);
		trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "b", bubbles: true }));
		await tick();
		expect(selected.current).toBe("banana");
	} finally {
		await unmount(view);
		clock.mockRestore();
	}
});
