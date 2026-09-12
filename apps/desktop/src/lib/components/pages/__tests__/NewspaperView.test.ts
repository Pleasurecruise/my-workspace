import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import NewspaperView from "../NewspaperView.svelte";
import type { KnowledgeDocument } from "../../../consumer";

const opener = vi.hoisted(() => ({ openUrl: vi.fn().mockResolvedValue(null) }));
vi.mock("@tauri-apps/plugin-opener", () => opener);

it("opens article links in the browser without navigating the reader", async () => {
	const issue: KnowledgeDocument = {
		id: "daily",
		slug: "daily",
		title: "Daily",
		summary: "News",
		tags: [],
		visibility: "private",
		contentHash: "hash",
		createdAt: "2026-09-12",
		updatedAt: "2026-09-12",
		newspaperEdition: "developer",
		source: "",
		toc: [],
		html: '<a href="https://example.com/news"><strong>Story</strong></a><a href="#section">Section</a><h2 id="section">Section</h2>',
	};
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(NewspaperView, {
		target,
		props: { documents: [issue], issues: { developer: "daily", personal: null }, loading: false },
	});
	await tick();
	const link = target.querySelector("a strong");
	if (!link) throw new Error("Missing article link");
	for (const type of ["click", "auxclick"]) {
		const event = new MouseEvent(type, {
			bubbles: true,
			cancelable: true,
			button: type === "auxclick" ? 1 : 0,
		});
		link.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(true);
	}
	expect(opener.openUrl).toHaveBeenCalledWith("https://example.com/news");
	expect(opener.openUrl).toHaveBeenCalledTimes(2);
	const anchor = target.querySelector('a[href="#section"]');
	if (!anchor) throw new Error("Missing section link");
	anchor.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
	expect(opener.openUrl).toHaveBeenCalledTimes(2);
	expect(target.querySelector(".copy")).not.toBeNull();
	await unmount(view);
	target.remove();
});
