import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, KnowledgeDocument } from "../../../consumer";
import KnowledgeView, { selectKnowledgeArticle } from "../KnowledgeView.svelte";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (!element) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("renders the index without reading bodies and ignores detail completion after leaving", async () => {
	const entry = {
		id: "lazy",
		slug: "lazy",
		title: "Lazy article",
		summary: "Summary",
		tags: [],
		visibility: "private" as const,
		contentHash: "v1",
		createdAt: "2026-09-12",
		updatedAt: "2026-09-12",
		newspaperEdition: null,
	};
	let complete!: (response: CommandResponse<KnowledgeDocument>) => void;
	const onread = vi.fn(
		() =>
			new Promise<CommandResponse<KnowledgeDocument>>((resolve) => {
				complete = resolve;
			}),
	);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(KnowledgeView, {
		target,
		props: { documents: [entry], loading: false, onread, oncreate: vi.fn(), onupdate: vi.fn() },
	});
	await tick();
	expect(target.textContent).toContain("Lazy article");
	expect(onread).not.toHaveBeenCalled();
	findElement<HTMLButtonElement>(target, '[data-knowledge-id="lazy"]').click();
	await tick();
	expect(onread).toHaveBeenCalledWith("lazy", "v1");
	expect(target.textContent).not.toContain("Loading");
	expect(
		findElement<HTMLButtonElement>(target, '[data-knowledge-id="lazy"]').getAttribute("aria-busy"),
	).toBe("true");
	await unmount(view);
	complete({ status: "ready", data: { ...entry, source: "Body", html: "<p>Body</p>", toc: [] } });
	await tick();
	const returned = mount(KnowledgeView, {
		target,
		props: { documents: [entry], loading: false, onread, oncreate: vi.fn(), onupdate: vi.fn() },
	});
	await tick();
	expect(target.querySelector(".reader")).toBeNull();
	expect(target.textContent).toContain("Lazy article");
	await unmount(returned);
	target.remove();
});

it("returns from a related article to its source before returning to the index", async () => {
	const source: KnowledgeDocument = {
		id: "source",
		slug: "source",
		title: "Source article",
		summary: "",
		tags: [],
		visibility: "private",
		contentHash: "hash",
		createdAt: "2026-09-12",
		updatedAt: "2026-09-12",
		newspaperEdition: null,
		source: "Body",
		html: '<a href="/articles/related">Related</a>',
		toc: [],
	};
	const related = {
		...source,
		id: "related",
		title: "Related article",
		html: "<p>Related body</p>",
	};
	expect(selectKnowledgeArticle(source)).toBeNull();
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(KnowledgeView, {
		target,
		props: { documents: [], loading: false, onread: vi.fn(), oncreate: vi.fn(), onupdate: vi.fn() },
	});
	await tick();
	invoke.mockResolvedValueOnce({ status: "ready", data: related });
	findElement<HTMLAnchorElement>(target, ".prose a").click();
	await vi.waitFor(() => expect(target.querySelector("h1")?.textContent).toBe("Related article"));
	findElement<HTMLButtonElement>(target, '[aria-label="Back to previous article"]').click();
	await tick();
	expect(target.querySelector("h1")?.textContent).toBe("Source article");
	expect(target.querySelector(".index")).toBeNull();
	findElement<HTMLButtonElement>(target, '[aria-label="Back to articles"]').click();
	await tick();
	expect(target.querySelector(".index")).not.toBeNull();
	expect(invoke).toHaveBeenCalledTimes(1);
	await unmount(view);
	target.remove();
	invoke.mockReset();
});

it("copies the canonical article link and reports clipboard failures", async () => {
	const clipboard = Object.getOwnPropertyDescriptor(navigator, "clipboard");
	const writeText = vi.fn().mockResolvedValue(undefined);
	Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
	const article: KnowledgeDocument = {
		id: "copy",
		slug: "article slug",
		title: "Copy",
		summary: "",
		tags: [],
		visibility: "private",
		contentHash: "hash",
		createdAt: "2026-09-12",
		updatedAt: "2026-09-12",
		newspaperEdition: null,
		source: "Body",
		html: "<p>Body</p>",
		toc: [],
	};
	expect(selectKnowledgeArticle(article)).toBeNull();
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(KnowledgeView, {
		target,
		props: { documents: [], loading: false, onread: vi.fn(), oncreate: vi.fn(), onupdate: vi.fn() },
	});
	await tick();
	findElement<HTMLButtonElement>(target, '[aria-label="Copy article link"]').click();
	await vi.waitFor(() =>
		expect(target.querySelector('[aria-label="Article link copied"]')).not.toBeNull(),
	);
	expect(writeText).toHaveBeenCalledWith("https://knowledge.you-find.me/articles/article%20slug");
	writeText.mockRejectedValueOnce(new Error("Denied"));
	findElement<HTMLButtonElement>(target, '[aria-label="Article link copied"]').click();
	await vi.waitFor(() =>
		expect(target.querySelector('[role="alert"]')?.textContent).toContain("Could not copy"),
	);
	findElement<HTMLButtonElement>(target, '[aria-label="Back to articles"]').click();
	await tick();
	await unmount(view);
	target.remove();
	if (clipboard) Object.defineProperty(navigator, "clipboard", clipboard);
	else Reflect.deleteProperty(navigator, "clipboard");
	vi.restoreAllMocks();
});

it("restores an article draft after navigation and preserves changes made during a save", async () => {
	let resolvePending: (response: CommandResponse<KnowledgeDocument>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const pending = new Promise<CommandResponse<KnowledgeDocument>>((resolve) => {
		resolvePending = resolve;
	});
	const other: KnowledgeDocument = {
		id: "other",
		slug: "other",
		title: "Other",
		summary: "Other",
		tags: [],
		visibility: "private",
		contentHash: "other-hash",
		createdAt: "2026-09-05",
		updatedAt: "2026-09-05",
		newspaperEdition: null,
		source: "Other",
		html: "<p>Other</p>",
		toc: [],
	};
	const props = {
		documents: [],
		loading: false,
		oncreate: vi.fn(() => pending),
		onupdate: vi.fn(),
		onread: vi.fn(),
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
	expect(selectKnowledgeArticle(other)).toContain("Finish or cancel");
	findElement<HTMLButtonElement>(target, ".editor-actions .save").click();
	expect(selectKnowledgeArticle(other)).toContain("Finish or cancel");
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
	findElement<HTMLButtonElement>(target, ".editor-actions button").click();
	await tick();
	await unmount(view);
	target.remove();
});
