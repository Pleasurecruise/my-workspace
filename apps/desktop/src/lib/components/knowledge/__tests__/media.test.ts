import { afterEach, expect, it, vi } from "vite-plus/test";
import { tick } from "svelte";
import { mediaPlayers } from "../media";

const cleanups: (() => void)[] = [];
afterEach(() => {
	for (const cleanup of cleanups.splice(0)) cleanup();
	vi.restoreAllMocks();
});

async function setup(kind = "audio") {
	const node = document.createElement("article");
	node.innerHTML = `<figure class="content-embed-media"><${kind} controls src="https://example.com/media.mp3" aria-label="Recording"></${kind}><figcaption>Caption</figcaption></figure>`;
	document.body.append(node);
	const media = node.querySelector<HTMLMediaElement>(kind)!;
	let paused = true;
	Object.defineProperty(media, "paused", { configurable: true, get: () => paused });
	Object.defineProperty(media, "duration", { configurable: true, value: 120 });
	const pause = vi.spyOn(media, "pause").mockImplementation(() => {
		paused = true;
		media.dispatchEvent(new Event("pause"));
	});
	const play = vi.spyOn(media, "play").mockImplementation(async () => {
		paused = false;
		media.dispatchEvent(new Event("play"));
	});
	const action = mediaPlayers(node, node.innerHTML);
	cleanups.push(() => {
		action.destroy();
		node.remove();
	});
	await tick();
	await tick();
	function button(label: string) {
		const result = node.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
		if (!result) throw new Error(`Missing ${label}`);
		return result;
	}
	return { node, media, action, play, pause, button };
}

it("replaces native controls and synchronizes playback, seek, mute and end state", async () => {
	const { node, media, play, button } = await setup();
	expect(media.controls).toBe(false);
	expect(media.style.display).toBe("none");
	expect(play).not.toHaveBeenCalled();
	button("播放").click();
	await tick();
	expect(play).toHaveBeenCalledOnce();
	await vi.waitFor(() => expect(button("暂停").disabled).toBe(false));
	button("暂停").click();
	await tick();
	expect(button("播放")).toBeDefined();
	const seek = node.querySelector<HTMLInputElement>('input[aria-label="播放进度"]')!;
	seek.value = "45";
	seek.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	expect(media.currentTime).toBe(45);
	expect(seek.getAttribute("aria-valuetext")).toBe("0:45 / 2:00");
	button("静音").click();
	media.dispatchEvent(new Event("volumechange"));
	await tick();
	expect(media.muted).toBe(true);
	expect(button("取消静音")).toBeDefined();
	const volume = node.querySelector<HTMLInputElement>('input[aria-label="音量"]')!;
	volume.value = "0.4";
	volume.dispatchEvent(new Event("input", { bubbles: true }));
	expect(media.volume).toBe(0.4);
	expect(media.muted).toBe(false);
	media.currentTime = 120;
	media.dispatchEvent(new Event("ended"));
	await tick();
	expect(seek.value).toBe("120");
	expect(node.querySelector("figcaption")?.textContent).toBe("Caption");
});

it("reports rejected playback and resource failures, and permits retry", async () => {
	const { node, media, play, button } = await setup();
	play.mockRejectedValueOnce(new Error("Unsupported format"));
	button("播放").click();
	await tick();
	await tick();
	expect(node.querySelector('[role="alert"]')?.textContent).toContain("无法开始播放");
	expect(button("播放").disabled).toBe(false);
	button("播放").click();
	await tick();
	expect(node.querySelector('[role="alert"]')).toBeNull();
	media.dispatchEvent(new Event("error"));
	await tick();
	expect(node.querySelector('[role="alert"]')?.textContent).toContain("无法播放此媒体");
});

it("keeps video visible and disables seeking before duration is known", async () => {
	const { node, media } = await setup("video");
	expect(media.style.display).toBe("");
	Object.defineProperty(media, "duration", { value: NaN });
	media.dispatchEvent(new Event("durationchange"));
	await tick();
	expect(node.querySelector<HTMLInputElement>('input[aria-label="播放进度"]')?.disabled).toBe(true);
});

it("stops detached media, restores native controls, and ignores stale enhancement requests", async () => {
	const { node, media, action, pause } = await setup();
	action.update("replacement");
	node.innerHTML = "<p>Another article</p>";
	action.update("newest");
	action.destroy();
	await tick();
	expect(pause).toHaveBeenCalledOnce();
	expect(media.controls).toBe(true);
	expect(media.style.display).toBe("");
	expect(node.querySelector("button")).toBeNull();
});

it("stops a pending play operation that resolves after leaving the reader", async () => {
	const { action, play, pause, button } = await setup();
	let finish!: () => void;
	play.mockImplementation(
		() =>
			new Promise<void>((resolve) => {
				finish = resolve;
			}),
	);
	button("播放").click();
	action.destroy();
	await tick();
	finish();
	await tick();
	expect(pause).toHaveBeenCalledTimes(2);
});

it("does not duplicate controls on content updates and enhances the replacement", async () => {
	const { node, media, action, pause } = await setup();
	const replacement =
		'<figure class="content-embed-media"><video controls src="https://example.com/clip.mp4"></video></figure>';
	node.innerHTML = replacement;
	action.update(replacement);
	await tick();
	await tick();
	action.update(replacement);
	await tick();
	expect(pause).toHaveBeenCalledOnce();
	expect(media.controls).toBe(true);
	expect(node.querySelectorAll('[role="group"]')).toHaveLength(1);
	expect(node.querySelector("video")?.controls).toBe(false);
});

it("requests fullscreen on the video frame so playback controls stay accessible", async () => {
	Object.defineProperty(document, "fullscreenEnabled", { configurable: true, value: true });
	cleanups.push(() => {
		Reflect.deleteProperty(document, "fullscreenEnabled");
	});
	const { node, button } = await setup("video");
	const frame = node.querySelector("figure")!;
	const request = vi.fn().mockResolvedValue(undefined);
	Object.defineProperty(frame, "requestFullscreen", { configurable: true, value: request });
	button("全屏").click();
	await tick();
	expect(request).toHaveBeenCalledOnce();
});

it("allows cancelling a slow play request without showing its late rejection", async () => {
	const { node, play, pause, button } = await setup();
	let reject!: (error: Error) => void;
	play.mockImplementation(
		() =>
			new Promise<void>((_resolve, fail) => {
				reject = fail;
			}),
	);
	button("播放").click();
	await tick();
	button("暂停").click();
	await tick();
	expect(pause).toHaveBeenCalledOnce();
	expect(button("播放").disabled).toBe(false);
	reject(new Error("Playback aborted"));
	await tick();
	expect(node.querySelector('[role="alert"]')).toBeNull();
});
