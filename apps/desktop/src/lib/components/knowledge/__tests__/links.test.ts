import { beforeEach, expect, it, vi } from "vite-plus/test";
import { openArticleLinks, preloadArticles } from "../links";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const opener = vi.hoisted(() => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => opener);
beforeEach(() => {
	opener.openUrl.mockReset();
	invoke.mockReset();
});

it("keeps the reader in place and reports browser failures", async () => {
	opener.openUrl.mockRejectedValue(new Error("Unavailable"));
	const node = document.createElement("article");
	node.innerHTML = '<a href="https://example.com">Story</a>';
	const report = vi.fn();
	const action = openArticleLinks(node, { onError: report, onOpen: vi.fn() });
	const link = node.querySelector("a");
	if (!link) throw new Error("Missing link");
	const event = new MouseEvent("click", { bubbles: true, cancelable: true });
	link.dispatchEvent(event);
	expect(event.defaultPrevented).toBe(true);
	await vi.waitFor(() =>
		expect(report).toHaveBeenLastCalledWith(
			"Could not open the link in your browser. Please try again.",
		),
	);
	action.destroy();
});

it("blocks unsupported and relative navigation while leaving fragment links local", () => {
	const node = document.createElement("article");
	const report = vi.fn();
	const action = openArticleLinks(node, { onError: report, onOpen: vi.fn() });
	for (const href of ["javascript:alert(1)", "file:///tmp/example", "/another-page"]) {
		node.innerHTML = `<a href="${href}">Story</a>`;
		const link = node.querySelector("a");
		if (!link) throw new Error("Missing link");
		const event = new MouseEvent("click", { bubbles: true, cancelable: true });
		link.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(true);
	}
	node.innerHTML = '<a href="#section">Section</a>';
	const fragment = node.querySelector("a");
	if (!fragment) throw new Error("Missing fragment");
	const event = new MouseEvent("click", { bubbles: true, cancelable: true });
	fragment.dispatchEvent(event);
	expect(event.defaultPrevented).toBe(false);
	expect(opener.openUrl).not.toHaveBeenCalled();
	action.destroy();
});

it.each(["next click", "destroy"])("ignores late failures after %s", async (transition) => {
	let reject: (error: Error) => void = () => {
		throw new Error("Promise not initialized");
	};
	const pending = new Promise<void>((_resolve, fail) => {
		reject = fail;
	});
	opener.openUrl.mockReturnValueOnce(pending).mockResolvedValue(undefined);
	const node = document.createElement("article");
	node.innerHTML = '<a href="https://example.com">Story</a>';
	const report = vi.fn();
	const action = openArticleLinks(node, { onError: report, onOpen: vi.fn() });
	const link = node.querySelector("a");
	if (!link) throw new Error("Missing link");
	link.click();
	if (transition === "next click") link.click();
	else action.destroy();
	report.mockClear();
	reject(new Error("Late failure"));
	await pending.catch(() => {});
	expect(report).not.toHaveBeenCalled();
	action.destroy();
});

it("reads an internal article without opening the browser and reports failures", async () => {
	const node = document.createElement("article");
	node.innerHTML = '<a href="/articles/not-in-list">Read</a>';
	const onError = vi.fn();
	const onOpen = vi.fn(() => null);
	const action = openArticleLinks(node, { onError, onOpen });
	const link = node.querySelector("a");
	if (!link) throw new Error("Missing link");
	const data = { id: "not-in-list" };
	invoke.mockResolvedValueOnce({ status: "ready", data });
	link.click();
	await vi.waitFor(() => expect(onOpen).toHaveBeenCalledWith(data));
	expect(invoke).toHaveBeenCalledWith("read_knowledge", { id: "not-in-list", expectedHash: null });
	expect(opener.openUrl).not.toHaveBeenCalled();
	invoke.mockResolvedValueOnce({ status: "failed", message: "Article not found" });
	link.click();
	await vi.waitFor(() => expect(onError).toHaveBeenLastCalledWith("Article not found"));
	expect(onOpen).toHaveBeenCalledTimes(1);
	action.destroy();
});

it.each(["next click", "destroy", "update", "fragment"])(
	"discards stale article reads after %s",
	async (transition) => {
		let settle: () => void = () => {
			throw new Error("Not initialized");
		};
		const pending = new Promise((resolve) => {
			settle = () => resolve({ status: "ready", data: { id: "old" } });
		});
		invoke.mockReturnValueOnce(pending).mockResolvedValue({ status: "failed", message: "Missing" });
		const node = document.createElement("article");
		node.innerHTML = '<a href="/articles/old">Old</a><a href="#section">Section</a>';
		const onOpen = vi.fn(() => null);
		const navigation = { onError: vi.fn(), onOpen };
		const action = openArticleLinks(node, navigation);
		const link = node.querySelector("a");
		if (!link) throw new Error("Missing link");
		link.click();
		if (transition === "next click") link.click();
		else if (transition === "destroy") action.destroy();
		else if (transition === "update") action.update(navigation);
		else node.querySelector<HTMLAnchorElement>('a[href="#section"]')?.click();
		settle();
		await pending;
		expect(onOpen).not.toHaveBeenCalled();
		action.destroy();
	},
);

it("preloads only sustained intent and cancels pending work on teardown", async () => {
	vi.useFakeTimers();
	invoke.mockResolvedValue(undefined);
	const node = document.createElement("section");
	node.innerHTML = '<button data-knowledge-id="article" data-content-hash="v2">Read</button>';
	const action = preloadArticles(node);
	const button = node.querySelector("button")!;
	button.dispatchEvent(new Event("pointerover", { bubbles: true }));
	await vi.advanceTimersByTimeAsync(59);
	expect(invoke).not.toHaveBeenCalled();
	button.dispatchEvent(new Event("pointerout", { bubbles: true }));
	await vi.advanceTimersByTimeAsync(100);
	expect(invoke).not.toHaveBeenCalled();
	button.dispatchEvent(new Event("focusin", { bubbles: true }));
	await vi.advanceTimersByTimeAsync(1);
	expect(invoke).toHaveBeenCalledWith("prefetch_knowledge", { id: "article", expectedHash: "v2" });
	button.dispatchEvent(new Event("pointerover", { bubbles: true }));
	action.destroy();
	await vi.advanceTimersByTimeAsync(100);
	expect(invoke).toHaveBeenCalledTimes(1);
	vi.useRealTimers();
});

it("warms visible entries before clicks with two concurrent requests and stops on leave", async () => {
	let intersect!: (entries: Partial<IntersectionObserverEntry>[]) => void;
	const disconnect = vi.fn();
	vi.stubGlobal(
		"IntersectionObserver",
		class {
			constructor(callback: typeof intersect) {
				intersect = callback;
			}
			observe = vi.fn();
			unobserve = vi.fn();
			disconnect = disconnect;
		},
	);
	const completions: (() => void)[] = [];
	invoke.mockImplementation(() => new Promise<void>((resolve) => completions.push(resolve)));
	const node = document.createElement("section");
	node.innerHTML = [1, 2, 3, 4]
		.map((id) => `<button data-knowledge-id="${id}">Article</button>`)
		.join("");
	const action = preloadArticles(node, true);
	intersect(Array.from(node.children, (target) => ({ target, isIntersecting: true })));
	expect(invoke).toHaveBeenCalledTimes(2);
	intersect([{ target: node.children[2], isIntersecting: false }]);
	completions[0]?.();
	await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(3));
	expect(invoke).toHaveBeenLastCalledWith("prefetch_knowledge", { id: "4", expectedHash: null });
	action.destroy();
	completions[1]?.();
	completions[2]?.();
	await Promise.resolve();
	await Promise.resolve();
	expect(invoke).toHaveBeenCalledTimes(3);
	expect(disconnect).toHaveBeenCalledOnce();
	vi.unstubAllGlobals();
});

it("opens an unresolved Knowledge URL card inside the application", async () => {
	const node = document.createElement("article");
	node.innerHTML =
		'<a class="content-embed-article" href="https://knowledge.you-find.me/articles/a-slug"><strong>Article title</strong><span>Article description</span></a>';
	const onOpen = vi.fn(() => null);
	const data = { id: "resolved-id" };
	invoke.mockResolvedValue({ status: "ready", data });
	const action = openArticleLinks(node, { onError: vi.fn(), onOpen });
	node.querySelector<HTMLAnchorElement>("a")?.click();
	await vi.waitFor(() => expect(onOpen).toHaveBeenCalledWith(data));
	expect(invoke).toHaveBeenCalledWith("read_knowledge", {
		id: "https://knowledge.you-find.me/articles/a-slug",
		expectedHash: null,
	});
	expect(opener.openUrl).not.toHaveBeenCalled();
	action.destroy();
});
