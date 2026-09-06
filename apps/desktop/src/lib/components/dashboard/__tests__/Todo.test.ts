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
				items: [{ id: "local", text: "Saved task", completed: false, details: null }],
				syncError: error,
			},
			error,
			loading: false,
			selectedDate: "2026-09-07",
			onadd: async () => true,
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
