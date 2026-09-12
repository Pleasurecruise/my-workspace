import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import Todo from "../Todo.svelte";

it("shows sync errors beside saved tasks", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const error = "Notion: ntn is not authenticated; run `ntn login` in Terminal";
	const view = mount(Todo, {
		target,
		props: {
			todos: {
				date: "2026-09-07",
				items: [
					{
						id: "local",
						text: "Saved task",
						completed: false,
						rollover: false,
						description: null,
						details: null,
					},
				],
				syncError: error,
			},
			error,
			loading: false,
			selectedDate: "2026-09-07",
			onadd: async () => true,
			onedit: async () => true,
			ontoggle: async () => {},
			ondelete: async () => {},
			onreorder: async () => true,
			onrollover: async () => {},
		},
	});
	try {
		await tick();
		expect(target.querySelector('[role="alert"]')?.textContent).toBe(error);
		expect(target.querySelector("ul")?.textContent).toContain("Saved task");
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("adds descriptions, edits completed tasks, and retains the editor after a failed save", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const added: Array<[string, string]> = [];
	const edited: Array<[string, string, string]> = [];
	const view = mount(Todo, {
		target,
		props: {
			todos: {
				date: "2026-09-10",
				syncError: null,
				items: [
					{
						id: "read",
						text: "Read",
						description: "Chapter one",
						completed: true,
						rollover: false,
						details: null,
					},
				],
			},
			error: null,
			loading: false,
			selectedDate: "2026-09-10",
			onadd: async (text, description) => {
				added.push([text, description]);
				return true;
			},
			onedit: async (id, text, description) => {
				edited.push([id, text, description]);
				return edited.length > 1;
			},
			ontoggle: async () => {},
			ondelete: async () => {},
			onreorder: async () => true,
			onrollover: async () => {},
		},
	});
	function click(label: string) {
		const button = target.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
		if (button === null) throw new Error(`Missing ${label}`);
		button.click();
	}
	function fill(selector: string, value: string) {
		const input = target.querySelector<HTMLInputElement | HTMLTextAreaElement>(selector);
		if (input === null) throw new Error(`Missing ${selector}`);
		input.value = value;
		input.dispatchEvent(new Event("input", { bubbles: true }));
	}
	try {
		await tick();
		fill("input[maxlength='120']", "Walk");
		click("Add description");
		await tick();
		fill("textarea", "Around the park");
		target
			.querySelector("form")
			?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
		await tick();
		await tick();
		expect(added).toEqual([["Walk", "Around the park"]]);
		expect(target.querySelector("input")?.value).toBe("");
		click("View details for Read");
		await tick();
		expect(target.textContent).toContain("Chapter one");
		click("Edit Todo");
		await tick();
		expect(target.querySelector("input")?.value).toBe("Read");
		fill("input", "Read more");
		fill("textarea", "Chapter two");
		target
			.querySelector("form")
			?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
		await tick();
		await tick();
		expect(edited).toEqual([["read", "Read more", "Chapter two"]]);
		expect(target.querySelector("textarea")?.value).toBe("Chapter two");
		expect(target.textContent).toContain("Done");
		target
			.querySelector("form")
			?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
		await tick();
		await tick();
		expect(target.querySelector("form")).toBeNull();
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("blocks duplicate submissions and preserves the draft after a failed creation", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const pending = deferred<boolean>();
	let submissions = 0;
	const view = mount(Todo, {
		target,
		props: {
			todos: null,
			error: null,
			loading: false,
			selectedDate: "2026-09-10",
			onadd: () => {
				submissions += 1;
				return pending.promise;
			},
			onedit: async () => true,
			ontoggle: async () => {},
			ondelete: async () => {},
			onreorder: async () => true,
			onrollover: async () => {},
		},
	});
	try {
		await tick();
		const input = target.querySelector("input");
		if (input === null) throw new Error("Missing title input");
		input.value = "First";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		target
			.querySelector("form")
			?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
		await tick();
		expect(input.disabled).toBe(true);
		target
			.querySelector("form")
			?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
		expect(submissions).toBe(1);
		pending.resolve(false);
		await tick();
		await tick();
		expect(input.value).toBe("First");
		expect(input.disabled).toBe(false);
	} finally {
		await unmount(view);
		target.remove();
	}
});

function deferred<T>() {
	let resolve: (value: T) => void = () => {
		throw new Error("Promise is not initialized");
	};
	const promise = new Promise<T>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}

it("reorders with the keyboard and preserves the list while a save is pending or fails", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const pending = deferred<boolean>();
	const calls: string[][] = [];
	const view = mount(Todo, {
		target,
		props: {
			todos: {
				date: "2026-09-12",
				syncError: null,
				items: ["First", "Second", "Third"].map((text) => ({
					id: text,
					text,
					completed: false,
					rollover: false,
					description: null,
					details: null,
				})),
			},
			error: null,
			loading: false,
			selectedDate: "2026-09-12",
			onadd: async () => true,
			onedit: async () => true,
			ontoggle: async () => {},
			ondelete: async () => {},
			onrollover: async () => {},
			onreorder: (ids) => {
				calls.push(ids);
				return pending.promise;
			},
		},
	});
	try {
		await tick();
		const handle = target.querySelector<HTMLButtonElement>('[aria-label="Reorder First"]');
		if (!handle) throw new Error("Missing drag handle");
		handle.focus();
		handle.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
		await tick();
		expect(calls).toEqual([["Second", "Third", "First"]]);
		expect(handle.disabled).toBe(true);
		expect(target.querySelector("li")?.textContent).toContain("First");
		handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
		expect(calls).toHaveLength(1);
		pending.resolve(false);
		await tick();
		await tick();
		await tick();
		expect(handle.disabled).toBe(false);
		expect(document.activeElement).toBe(handle);
		expect(target.querySelector("li")?.textContent).toContain("First");
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("uses the final pointer position, cancels dragging, and recovers from a rejected save", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const onreorder = vi
		.fn()
		.mockRejectedValueOnce(new Error("Disconnected"))
		.mockResolvedValue(true);
	const frames = vi.spyOn(window, "requestAnimationFrame").mockReturnValue(1);
	const view = mount(Todo, {
		target,
		props: {
			todos: {
				date: "2026-09-12",
				syncError: null,
				items: ["First", "Second", "Third"].map((text) => ({
					id: text,
					text,
					completed: false,
					rollover: false,
					description: null,
					details: null,
				})),
			},
			error: null,
			loading: false,
			selectedDate: "2026-09-12",
			onadd: async () => true,
			onedit: async () => true,
			ontoggle: async () => {},
			ondelete: async () => {},
			onreorder,
			onrollover: async () => {},
		},
	});
	try {
		await tick();
		const handle = target.querySelector<HTMLButtonElement>('[aria-label="Reorder First"]');
		if (!handle) throw new Error("Missing drag handle");
		Object.defineProperties(handle, {
			setPointerCapture: { value: () => {} },
			hasPointerCapture: { value: () => false },
		});
		target.querySelectorAll("li").forEach((row, index) => {
			vi.spyOn(row, "getBoundingClientRect").mockReturnValue(new DOMRect(0, index * 30, 200, 30));
		});
		frames.mockClear();
		handle.dispatchEvent(
			new PointerEvent("pointerdown", { bubbles: true, button: 0, clientY: 15, pointerId: 1 }),
		);
		expect(frames).not.toHaveBeenCalled();
		handle.dispatchEvent(
			new PointerEvent("pointermove", { bubbles: true, clientY: 45, pointerId: 1 }),
		);
		handle.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
		handle.dispatchEvent(
			new PointerEvent("pointerup", { bubbles: true, clientY: 75, pointerId: 1 }),
		);
		expect(onreorder).not.toHaveBeenCalled();
		handle.dispatchEvent(
			new PointerEvent("pointerdown", { bubbles: true, button: 0, clientY: 15, pointerId: 2 }),
		);
		handle.dispatchEvent(
			new PointerEvent("pointermove", { bubbles: true, clientY: 45, pointerId: 2 }),
		);
		handle.dispatchEvent(
			new PointerEvent("pointerup", { bubbles: true, clientY: 75, pointerId: 2 }),
		);
		await tick();
		await tick();
		await tick();
		expect(onreorder).toHaveBeenCalledWith(["Second", "Third", "First"]);
		expect(handle.disabled).toBe(false);
		expect(target.querySelector('[role="alert"]')?.textContent).toContain("Could not save");
		expect(target.querySelector("li")?.textContent).toContain("First");
		handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
		await tick();
		await tick();
		expect(onreorder).toHaveBeenCalledTimes(2);
		expect(target.querySelector('[role="alert"]')).toBeNull();
	} finally {
		await unmount(view);
		target.remove();
		vi.restoreAllMocks();
	}
});

it("keeps carry-forward separate from completion and reflects only saved preferences", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const onrollover = vi.fn(async () => {});
	const ontoggle = vi.fn(async () => {});
	const view = mount(Todo, {
		target,
		props: {
			todos: {
				date: "2026-09-12",
				syncError: null,
				items: [
					{
						id: "read",
						text: "Read",
						description: null,
						completed: false,
						rollover: false,
						details: null,
					},
				],
			},
			error: null,
			loading: false,
			selectedDate: "2026-09-12",
			onadd: async () => true,
			onedit: async () => true,
			ondelete: async () => {},
			onreorder: async () => true,
			onrollover,
			ontoggle,
		},
	});
	try {
		await tick();
		expect(target.querySelector(".rollover-option")).toBeNull();
		expect(target.querySelectorAll('input[type="checkbox"]')).toHaveLength(1);
		target.querySelector<HTMLButtonElement>('[aria-label="View details for Read"]')?.click();
		await tick();
		expect(
			target.querySelector(".todo-detail")?.lastElementChild?.classList.contains("rollover-option"),
		).toBe(true);
		expect(target.querySelector(".rollover-option")?.textContent).toContain(
			"Move to the next day if unfinished",
		);
		const checkbox = target.querySelector<HTMLInputElement>(
			'[aria-label="Carry Read forward if unfinished"]',
		);
		if (!checkbox) throw new Error("Missing carry-forward checkbox");
		expect(checkbox.checked).toBe(false);
		checkbox.click();
		await tick();
		expect(onrollover).toHaveBeenCalledWith("read", true);
		expect(ontoggle).not.toHaveBeenCalled();
		expect(checkbox.checked).toBe(false);
		expect(target.querySelectorAll('input[type="checkbox"]')).toHaveLength(1);
	} finally {
		await unmount(view);
		target.remove();
	}
});
