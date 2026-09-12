import { mount, tick, unmount } from "svelte";
import MediaControls from "./MediaControls.svelte";

// Enhance compiled HTML in both readers while preserving the compiler's no-JS fallback.
export function mediaPlayers(node: HTMLElement, html: string) {
	let content = html;
	let generation = 0;
	let dispose: (() => void)[] = [];

	function clear() {
		for (const cleanup of dispose) cleanup();
		dispose = [];
	}

	async function enhance() {
		const request = ++generation;
		clear();
		await tick();
		if (request !== generation) return;
		for (const media of node.querySelectorAll<HTMLMediaElement>(
			".content-embed-media > audio, .content-embed-media > video",
		)) {
			const frame = media.parentElement;
			if (!frame) continue;
			const controls = media.controls;
			const display = media.style.display;
			const target = document.createElement("div");
			media.after(target);
			const component = mount(MediaControls, { target, props: { media, frame } });
			media.controls = false;
			if (media instanceof HTMLAudioElement) media.style.display = "none";
			dispose.push(() => {
				media.pause();
				void unmount(component);
				target.remove();
				media.controls = controls;
				media.style.display = display;
			});
		}
	}
	void enhance();
	return {
		update(next: string) {
			if (content === next) return;
			content = next;
			void enhance();
		},
		destroy() {
			generation++;
			clear();
		},
	};
}
