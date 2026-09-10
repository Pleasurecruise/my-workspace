import { expect, it } from "vite-plus/test";
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
					{ id: "local", text: "Saved task", completed: false, description: null, details: null },
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
					{ id: "read", text: "Read", description: "Chapter one", completed: true, details: null },
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
