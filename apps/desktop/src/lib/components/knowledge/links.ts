import { invoke } from "@tauri-apps/api/core";
import type { CommandResponse, KnowledgeDocument } from "../../consumer";
import { openUrl } from "@tauri-apps/plugin-opener";

// Desktop routes carry IDs; Rust extracts article IDs from canonical web URLs.
type ArticleNavigation = {
	onError: (message: string | null) => void;
	onOpen: (document: KnowledgeDocument, fragment?: string) => string | null;
};
export function openArticleLinks(node: HTMLElement, navigation: ArticleNavigation) {
	let generation = 0;
	const preload = preloadArticles(node);
	function openLink(event: MouseEvent) {
		if (event.defaultPrevented || (event.type === "auxclick" && event.button !== 1)) return;
		const anchor = event.composedPath().find((target) => target instanceof HTMLAnchorElement);
		if (!(anchor instanceof HTMLAnchorElement)) return;
		const href = anchor.getAttribute("href");
		if (href === null) return;
		if (href.startsWith("#")) {
			generation++;
			navigation.onError(null);
			return;
		}
		event.preventDefault();
		const request = ++generation;
		navigation.onError(null);
		const article = /^\/articles\/([^/?#]+)(?:#[^\s]*)?$/.exec(href);
		const cardUrl =
			anchor.classList.contains("content-embed-article") &&
			URL.canParse(href) &&
			new URL(href).origin === "https://knowledge.you-find.me"
				? href
				: null;
		if (article !== null || cardUrl !== null) {
			void invoke<CommandResponse<KnowledgeDocument>>("read_knowledge", {
				id: cardUrl ?? article?.[1],
				expectedHash: null,
			})
				.then((response) => {
					if (request !== generation) return;
					if (response.status === "failed") navigation.onError(response.message);
					else {
						const fragment = new URL(href, "https://knowledge.you-find.me").hash.slice(1);
						const error = fragment
							? navigation.onOpen(response.data, fragment)
							: navigation.onOpen(response.data);
						navigation.onError(error);
					}
				})
				.catch(() => {
					if (request === generation)
						navigation.onError("Could not open the article. Please try again.");
				});
			return;
		}
		if (!URL.canParse(href) || !["https:", "http:", "mailto:"].includes(new URL(href).protocol)) {
			navigation.onError(
				"This article link cannot be opened. Use an absolute web or email address.",
			);
			return;
		}
		void openUrl(href).catch(() => {
			if (request === generation) {
				navigation.onError("Could not open the link in your browser. Please try again.");
			}
		});
	}
	node.addEventListener("click", openLink);
	node.addEventListener("auxclick", openLink);
	return {
		update(next: ArticleNavigation) {
			generation++;
			navigation = next;
		},
		destroy() {
			generation++;
			preload.destroy();
			node.removeEventListener("click", openLink);
			node.removeEventListener("auxclick", openLink);
		},
	};
}

// Visible index entries warm after rendering; reader links warm on explicit intent.
export function preloadArticles(node: HTMLElement, visible = false) {
	let destroyed = false;
	let active = 0;
	const queue = new Set<HTMLElement>();
	const observed = new WeakSet<Element>();
	function drain() {
		if (destroyed) return;
		for (const target of queue) {
			if (active >= 2) break;
			queue.delete(target);
			if (!node.contains(target)) continue;
			observer?.unobserve(target);
			active++;
			void invoke("prefetch_knowledge", {
				id: target.dataset.knowledgeId,
				expectedHash: target.dataset.contentHash ?? null,
			})
				.catch(() => {})
				.finally(() => {
					active--;
					drain();
				});
		}
	}
	const observer =
		visible && typeof IntersectionObserver !== "undefined"
			? new IntersectionObserver(
					(entries) => {
						for (const entry of entries) {
							if (entry.isIntersecting && entry.target instanceof HTMLElement) {
								queue.add(entry.target);
							} else if (entry.target instanceof HTMLElement) {
								queue.delete(entry.target);
							}
						}
						drain();
					},
					{ rootMargin: "120px" },
				)
			: null;
	function observe() {
		for (const target of node.querySelectorAll("[data-knowledge-id]")) {
			if (!observed.has(target)) {
				observed.add(target);
				observer?.observe(target);
			}
		}
	}
	const mutations = observer === null ? null : new MutationObserver(observe);
	mutations?.observe(node, { childList: true, subtree: true });
	observe();
	let timer: ReturnType<typeof setTimeout> | null = null;
	function cancel() {
		if (timer !== null) clearTimeout(timer);
		timer = null;
	}
	function preload(event: Event) {
		if (!(event.target instanceof Element)) return;
		const target = event.target.closest<HTMLElement>("[data-knowledge-id], a");
		if (target === null || !node.contains(target)) return;
		const id =
			target.dataset.knowledgeId ??
			/^\/articles\/([^/?#]+)(?:#[^\s]*)?$/.exec(target.getAttribute("href") ?? "")?.[1];
		if (!id) return;
		cancel();
		const expectedHash = target.dataset.contentHash ?? null;
		timer = setTimeout(
			() => {
				timer = null;
				// Speculation is optional; an explicit click retries and reports errors.
				void invoke("prefetch_knowledge", { id, expectedHash }).catch(() => {});
			},
			event.type === "pointerover" ? 60 : 0,
		);
	}
	node.addEventListener("pointerover", preload);
	node.addEventListener("pointerout", cancel);
	node.addEventListener("focusin", preload);
	node.addEventListener("focusout", cancel);
	node.addEventListener("touchstart", preload, { passive: true });
	return {
		destroy() {
			destroyed = true;
			queue.clear();
			observer?.disconnect();
			mutations?.disconnect();
			cancel();
			node.removeEventListener("pointerover", preload);
			node.removeEventListener("pointerout", cancel);
			node.removeEventListener("focusin", preload);
			node.removeEventListener("focusout", cancel);
			node.removeEventListener("touchstart", preload);
		},
	};
}
