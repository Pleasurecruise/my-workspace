import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import NewspaperView from "../NewspaperView.svelte";
import type { NewspaperDaily } from "@/lib/contracts/newspaper";

const opener = vi.hoisted(() => ({ openUrl: vi.fn().mockResolvedValue(null) }));
vi.mock("@tauri-apps/plugin-opener", () => opener);

const daily: NewspaperDaily = {
	date: "2026-10-07",
	generatedAt: "2026-10-07T00:00:29.119Z",
	url: "https://aihot.news/daily/2026-10-07",
	lead: { title: "Lead story", paragraph: "Lead paragraph" },
	sections: [
		{
			label: "模型发布/更新",
			items: [
				{ title: "Model", summary: "Summary", source: "Blog", url: "https://example.com/model" },
			],
		},
	],
	flashes: [
		{
			title: "Flash",
			source: "RSS",
			url: "https://example.com/flash",
			publishedAt: "2026-10-06T17:32:00.000Z",
		},
	],
};

it("renders the daily and opens its links in the browser", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(NewspaperView, {
		target,
		props: { daily, error: null, loading: false, onretry: vi.fn() },
	});
	await tick();
	expect(target.querySelector(".cover h2")?.textContent).toBe("Lead story");
	expect(target.querySelector('[aria-label="模型发布/更新"] p')?.textContent).toBe("Summary");
	for (const type of ["click", "auxclick"]) {
		const link = target.querySelector<HTMLAnchorElement>('a[href="https://example.com/model"]');
		if (!link) throw new Error("Missing item link");
		const event = new MouseEvent(type, {
			bubbles: true,
			cancelable: true,
			button: type === "auxclick" ? 1 : 0,
		});
		link.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(true);
	}
	expect(opener.openUrl).toHaveBeenCalledWith("https://example.com/model");
	expect(target.querySelector('[aria-label="快讯"] a')?.textContent).toBe("Flash");
	await unmount(view);
	target.remove();
});

it("offers a retry when the first read fails", async () => {
	const onretry = vi.fn();
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(NewspaperView, {
		target,
		props: { daily: null, error: "AIHOT rate limit reached.", loading: false, onretry },
	});
	await tick();
	expect(target.querySelector('[role="alert"]')?.textContent).toBe("AIHOT rate limit reached.");
	target.querySelector<HTMLButtonElement>(".empty button")?.click();
	expect(onretry).toHaveBeenCalledOnce();
	await unmount(view);
	target.remove();
});
