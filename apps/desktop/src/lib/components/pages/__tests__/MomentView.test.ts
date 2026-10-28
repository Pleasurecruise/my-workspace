import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { PhotoItem } from "@/lib/contracts/content";
import MomentView from "../MomentView.svelte";

vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (path: string) => path }));

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (!element) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("restores the share icon after copying a photo link", async () => {
	const clipboard = Object.getOwnPropertyDescriptor(navigator, "clipboard");
	const writeText = vi.fn().mockResolvedValue(undefined);
	Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
	const photo: PhotoItem = {
		id: "harbour",
		url: "https://example.com/harbour.jpg",
		thumbnailUrl: "https://example.com/harbour-thumb.jpg",
		r2Key: "photos/harbour.jpg",
		thumbnailR2Key: "photos/harbour-thumb.jpg",
		thumbHash: null,
		title: "Harbour",
		width: 1200,
		height: 800,
		aspectRatio: 1.5,
		tags: [],
		date: "2026-10-01",
		description: null,
		size: null,
		format: null,
		geo: null,
	};
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(MomentView, {
		target,
		props: {
			photos: [photo],
			tags: [],
			total: 1,
			onupload: vi.fn(),
			onupdate: vi.fn(),
			ondelete: vi.fn(),
		},
	});
	await tick();
	findElement<HTMLButtonElement>(target, '[aria-label="Open Harbour"]').click();
	await tick();
	vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
	const share = findElement<HTMLButtonElement>(document.body, '[aria-label="Share photo"]');
	share.click();
	await vi.advanceTimersByTimeAsync(0);
	expect(writeText).toHaveBeenCalledWith("https://moment.you-find.me/photos/harbour");
	expect(share.title).toBe("Link copied");
	await vi.advanceTimersByTimeAsync(2000);
	expect(share.title).toBe("Share photo");
	vi.useRealTimers();
	await unmount(view);
	target.remove();
	if (clipboard) Object.defineProperty(navigator, "clipboard", clipboard);
	else Reflect.deleteProperty(navigator, "clipboard");
	vi.restoreAllMocks();
});
