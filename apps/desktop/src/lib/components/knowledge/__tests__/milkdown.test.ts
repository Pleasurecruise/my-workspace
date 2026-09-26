import { expect, it, vi } from "vite-plus/test";
import { TextSelection } from "@milkdown/kit/prose/state";
import { createEditor } from "../milkdown";
import type { MarkdownSpan } from "../../../consumer";
import fixtures from "@workspace/crates/cms-core/tests/fixtures/editor.json";

async function setup(source: string, ranges: MarkdownSpan[] = [], maxLength = 500_000) {
	const root = document.createElement("div");
	document.body.append(root);
	const changes = vi.fn();
	const limit = vi.fn();
	const editor = await createEditor(root, {
		maxLength,
		onChange: changes,
		onSelection: () => {},
		onLimit: limit,
	});
	const candidate = editor.setMarkdown(source, ranges);
	return {
		root,
		editor,
		changes,
		limit,
		candidate,
		async dispose() {
			await editor.milkdown.destroy();
			root.remove();
		},
	};
}

it.each(fixtures)("preserves $name through actual Milkdown", async (fixture) => {
	const context = await setup(fixture.source, fixture.ranges);
	try {
		expect(context.candidate).toBe(fixture.candidate);
		expect(context.changes).not.toHaveBeenCalled();
		if (fixture.protected.length) {
			const { view } = context.editor;
			// Editing the final ordinary paragraph must preserve every preceding source block.
			view.dispatch(view.state.tr.insertText(" edited", view.state.doc.content.size - 1));
			const source = context.changes.mock.lastCall?.[0];
			expect(source).toContain("After edited");
			for (const text of fixture.protected) expect(source).toContain(text);
		}
	} finally {
		await context.dispose();
	}
});

it("renders images, tables, footnotes and clickable task checkboxes", async () => {
	const context = await setup(
		'![Alt](https://example.com/image.png "Caption")\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n- [ ] Todo\n\nNote[^1]\n\n[^1]: Footnote\n',
	);
	try {
		expect(context.root.querySelector('img[alt="Alt"]')?.getAttribute("src")).toBe(
			"https://example.com/image.png",
		);
		expect(context.root.querySelectorAll("td")).toHaveLength(2);
		const checkbox = context.root.querySelector<HTMLInputElement>('input[type="checkbox"]');
		if (checkbox === null) throw new Error("Task checkbox missing");
		checkbox.click();
		expect(context.changes.mock.lastCall?.[0]).toContain("[x] Todo");
		expect(context.changes.mock.lastCall?.[0]).toContain("[^1]: Footnote");
	} finally {
		await context.dispose();
	}
});

it("edits source blocks literally and never executes their HTML", async () => {
	const source = '<script>alert("no")</script>\n\nAfter';
	const context = await setup(source, [{ start: 0, end: source.indexOf("\n") }]);
	try {
		expect(context.root.querySelector("script")).toBeNull();
		const { view } = context.editor;
		view.dispatch(view.state.tr.insertText(" changed", source.indexOf("</script>") + 1));
		expect(context.changes.mock.lastCall?.[0]).toContain('<script>alert("no") changed</script>');
	} finally {
		await context.dispose();
	}
});

it("updates synchronously, supports undo/redo and resets history on source reload", async () => {
	const context = await setup("Original");
	try {
		const { editor, changes } = context;
		editor.view.dispatch(editor.view.state.tr.insertText(" edited", 9));
		expect(changes).toHaveBeenLastCalledWith("Original edited\n");
		editor.execute("undo");
		expect(changes).toHaveBeenLastCalledWith("Original\n");
		editor.execute("redo");
		expect(changes).toHaveBeenLastCalledWith("Original edited\n");
		editor.setMarkdown("Replacement", []);
		changes.mockClear();
		editor.execute("undo");
		expect(changes).not.toHaveBeenCalled();
		expect(editor.view.state.doc.textContent).toBe("Replacement");
	} finally {
		await context.dispose();
	}
});

it("rejects edits above the limit while allowing deletions", async () => {
	const context = await setup("1234", [], 5);
	try {
		const { view } = context.editor;
		view.dispatch(view.state.tr.insertText("5", 5));
		expect(context.limit).toHaveBeenCalledOnce();
		expect(context.changes).not.toHaveBeenCalled();
		expect(view.state.doc.textContent).toBe("1234");
		view.dispatch(view.state.tr.delete(1, 2));
		expect(context.changes).toHaveBeenLastCalledWith("234\n");
	} finally {
		await context.dispose();
	}
});

it("keeps formatting commands and task-list conversion usable", async () => {
	const context = await setup("Hello");
	try {
		const { editor } = context;
		editor.view.dispatch(
			editor.view.state.tr.setSelection(TextSelection.create(editor.view.state.doc, 1, 6)),
		);
		editor.execute("bold");
		expect(context.changes.mock.lastCall?.[0]).toContain("**Hello**");
		editor.execute("heading");
		expect(context.changes.mock.lastCall?.[0]).toContain("## **Hello**");
		editor.setMarkdown("- Todo", []);
		editor.execute("task");
		expect(context.changes.mock.lastCall?.[0]).toBe("* [ ] Todo\n");
		editor.execute("task");
		expect(context.changes.mock.lastCall?.[0]).toBe("* Todo\n");
	} finally {
		await context.dispose();
	}
});

it("removes a whole formatted link without touching its neighbor", async () => {
	const context = await setup(
		"[**bold** plain](https://example.com)[next](https://other.example.com)",
	);
	try {
		const { editor } = context;
		editor.view.dispatch(
			editor.view.state.tr.setSelection(TextSelection.create(editor.view.state.doc, 3)),
		);
		editor.setLink(null);
		expect(context.changes.mock.lastCall?.[0]).toBe(
			"**bold** plain[next](https://other.example.com)\n",
		);
	} finally {
		await context.dispose();
	}
});

it("converts existing lists instead of wrapping another list", async () => {
	const context = await setup("- One\n- Two");
	try {
		context.editor.execute("orderedList");
		expect(context.changes.mock.lastCall?.[0]).toBe("1. One\n2. Two\n");
		context.editor.execute("bulletList");
		expect(context.changes.mock.lastCall?.[0]).toBe("* One\n* Two\n");
	} finally {
		await context.dispose();
	}
});

it("removes a blockquote without flattening its nested list", async () => {
	const context = await setup("> - One\n> - Two");
	try {
		context.editor.execute("blockquote");
		expect(context.changes.mock.lastCall?.[0]).toBe("* One\n* Two\n");
	} finally {
		await context.dispose();
	}
});
