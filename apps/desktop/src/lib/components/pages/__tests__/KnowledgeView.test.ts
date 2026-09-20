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
	complete({
		status: "ready",
		data: {
			...entry,
			source: "Body",
			html: "<p>Body</p>",
			toc: [],
			stats: { wordCount: 1, readingMinutes: 1 },
		},
	});
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
		title: "Source article",
		summary: "",
		tags: [],
		visibility: "private",
		contentHash: "hash",
		createdAt: "2026-09-12",
		updatedAt: "2026-09-12",
		newspaperEdition: null,
		source: "Body",
		html: '<a href="/articles/related#%63hapter">Related</a>',
		toc: [],
		stats: { wordCount: 450, readingMinutes: 3 },
	};
	const related = {
		...source,
		id: "related",
		title: "Related article",
		html: '<h2 id="chapter">Related body</h2>',
	};
	expect(selectKnowledgeArticle(source)).toBeNull();
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(KnowledgeView, {
		target,
		props: { documents: [], loading: false, onread: vi.fn(), oncreate: vi.fn(), onupdate: vi.fn() },
	});
	await tick();
	expect(findElement<HTMLElement>(target, ".stats").textContent).toContain("450");
	expect(findElement<HTMLElement>(target, ".stats").textContent).toContain("3 min");
	const scroll = vi.fn();
	const previousScroll = HTMLElement.prototype.scrollIntoView;
	HTMLElement.prototype.scrollIntoView = scroll;
	invoke.mockResolvedValueOnce({ status: "ready", data: related });
	findElement<HTMLAnchorElement>(target, ".prose a").click();
	await vi.waitFor(() => expect(target.querySelector("h1")?.textContent).toBe("Related article"));
	expect(scroll).toHaveBeenCalledWith({ behavior: "instant", block: "start" });
	HTMLElement.prototype.scrollIntoView = previousScroll;
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
		stats: { wordCount: 1, readingMinutes: 1 },
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
	expect(writeText).toHaveBeenCalledWith(`https://knowledge.you-find.me/articles/${article.id}`);
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
		stats: { wordCount: 1, readingMinutes: 1 },
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
	body.value = "Revised body\n\n```embed:stock\ncode: MSFT\n```";
	body.dispatchEvent(new Event("input", { bubbles: true }));
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
			stats: { wordCount: 1, readingMinutes: 1 },
		},
	});
	await vi.waitFor(() =>
		expect(findElement<HTMLButtonElement>(target, ".editor-actions .save").disabled).toBe(false),
	);
	expect(findElement<HTMLInputElement>(target, '[placeholder="Untitled knowledge"]').value).toBe(
		"Revised title",
	);
	expect(
		findElement<HTMLTextAreaElement>(target, '[aria-label="Article Markdown source"]').value,
	).toBe("Revised body\n\n```embed:stock\ncode: MSFT\n```");
	expect(target.querySelector(".reader")).toBeNull();
	findElement<HTMLButtonElement>(target, ".editor-actions button").click();
	await tick();
	await unmount(view);
	target.remove();
});

it("sends both article version fields when saving and retains a rejected draft", async () => {
	const article: KnowledgeDocument = {
		id: "019c1234-1234-7000-8000-123456789abc",
		title: "Versioned article",
		summary: "Summary",
		tags: [],
		visibility: "private",
		contentHash: "a".repeat(64),
		createdAt: "2026-09-20T10:00:00.000Z",
		updatedAt: "2026-09-20T11:00:00.000Z",
		newspaperEdition: null,
		source: "Body",
		html: "<p>Body</p>",
		toc: [],
		stats: { wordCount: 1, readingMinutes: 1 },
	};
	const onupdate = vi
		.fn()
		.mockResolvedValue({ status: "failed", message: "Article changed while saving" });
	expect(selectKnowledgeArticle(article)).toBeNull();
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(KnowledgeView, {
		target,
		props: { documents: [article], loading: false, onread: vi.fn(), oncreate: vi.fn(), onupdate },
	});
	try {
		await tick();
		findElement<HTMLButtonElement>(target, '[aria-label="Edit article"]').click();
		await tick();
		findElement<HTMLButtonElement>(target, ".editor-actions .save").click();
		await tick();
		expect(onupdate).toHaveBeenCalledWith(article.id, {
			title: article.title,
			summary: article.summary,
			body: article.source,
			tags: [],
			expectedHash: article.contentHash,
			expectedUpdatedAt: article.updatedAt,
			visibility: "private",
		});
		await vi.waitFor(() => expect(target.textContent).toContain("Article changed while saving"));
		expect(target.querySelector(".editor")).not.toBeNull();
	} finally {
		findElement<HTMLButtonElement>(target, ".editor-actions button").click();
		await tick();
		await unmount(view);
		target.remove();
	}
});
