import { openUrl } from "@tauri-apps/plugin-opener";

// Compiled articles share this browser boundary in Knowledge and Newspaper.
export function openArticleLinks(node: HTMLElement, onError: (message: string | null) => void) {
	let generation = 0;
	function openLink(event: MouseEvent) {
		if (event.defaultPrevented || (event.type === "auxclick" && event.button !== 1)) return;
		const anchor = event.composedPath().find((target) => target instanceof HTMLAnchorElement);
		if (!(anchor instanceof HTMLAnchorElement)) return;
		const href = anchor.getAttribute("href");
		if (href === null || href.startsWith("#")) return;
		event.preventDefault();
		const request = ++generation;
		onError(null);
		if (!URL.canParse(href) || !["https:", "http:", "mailto:"].includes(new URL(href).protocol)) {
			onError("This article link cannot be opened. Use an absolute web or email address.");
			return;
		}
		void openUrl(href).catch(() => {
			if (request === generation) {
				onError("Could not open the link in your browser. Please try again.");
			}
		});
	}
	node.addEventListener("click", openLink);
	node.addEventListener("auxclick", openLink);
	return {
		destroy() {
			generation++;
			node.removeEventListener("click", openLink);
			node.removeEventListener("auxclick", openLink);
		},
	};
}
