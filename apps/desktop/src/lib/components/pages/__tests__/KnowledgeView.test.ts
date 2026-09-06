import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, KnowledgeDocument } from "../../../consumer";
import KnowledgeView from "../KnowledgeView.svelte";

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (!element) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("restores an article draft after navigation and preserves changes made during a save", async () => {
	let resolvePending: (response: CommandResponse<KnowledgeDocument>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const pending = new Promise<CommandResponse<KnowledgeDocument>>((resolve) => {
		resolvePending = resolve;
	});
	const props = {
		documents: [],
		loading: false,
		oncreate: vi.fn(() => pending),
		onupdate: vi.fn(),
	};
	const target = document.createElement("div");
	document.body.append(target);
	let view = mount(KnowledgeView, { target, props });
	await tick();
	findElement<HTMLButtonElement>(target, ".index-header button").click();
	await tick();
	const title = findElement<HTMLInputElement>(target, '[placeholder="Untitled knowledge"]');
	const summary = findElement<HTMLInputElement>(target, '[placeholder="A short summary"]');
	title.value = "Draft title";
	title.dispatchEvent(new Event("input", { bubbles: true }));
	summary.value = "Draft summary";
	summary.dispatchEvent(new Event("input", { bubbles: true }));
	findElement<HTMLButtonElement>(target, ".mode-switch button:last-child").click();
	await tick();
	const body = findElement<HTMLTextAreaElement>(target, '[aria-label="Article Markdown source"]');
	body.value = "Draft body";
	body.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	findElement<HTMLButtonElement>(target, ".editor-actions .save").click();
	expect(props.oncreate).toHaveBeenCalledWith({
		title: "Draft title",
		summary: "Draft summary",
		body: "Draft body",
		tags: [],
	});
	title.value = "Revised title";
	title.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	await unmount(view);
	view = mount(KnowledgeView, { target, props });
	await tick();
	expect(findElement<HTMLInputElement>(target, '[placeholder="Untitled knowledge"]').value).toBe(
		"Revised title",
	);
	resolvePending({
		status: "ready",
		data: {
			id: "article-1",
			slug: "article-1",
			title: "Draft title",
			summary: "Draft summary",
			tags: [],
			visibility: "private",
			contentHash: "saved-hash",
			createdAt: "2026-09-05",
			updatedAt: "2026-09-05",
			newspaperEdition: null,
			source: "Draft body",
			html: "<p>Draft body</p>",
			toc: [],
		},
	});
	await vi.waitFor(() =>
		expect(findElement<HTMLButtonElement>(target, ".editor-actions .save").disabled).toBe(false),
	);
	expect(findElement<HTMLInputElement>(target, '[placeholder="Untitled knowledge"]').value).toBe(
		"Revised title",
	);
	expect(target.querySelector(".reader")).toBeNull();
	await unmount(view);
	target.remove();
});
