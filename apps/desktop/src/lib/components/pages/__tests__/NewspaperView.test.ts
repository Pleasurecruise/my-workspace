import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import NewspaperView from "../NewspaperView.svelte";
import type { KnowledgeDocument } from "../../../consumer";

const opener = vi.hoisted(() => ({ openUrl: vi.fn().mockResolvedValue(null) }));
vi.mock("@tauri-apps/plugin-opener", () => opener);

const issue: KnowledgeDocument = {
	id: "daily",
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
	stats: { wordCount: 1, readingMinutes: 1 },
	html: '<a href="https://example.com/news"><strong>Story</strong></a><a href="#section">Section</a><h2 id="section">Section</h2>',
};

it("opens article links in the browser without navigating the reader", async () => {
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(NewspaperView, {
		target,
		props: {
			documents: [issue],
			issues: { developer: "daily", personal: null },
			loading: false,
			onopenarticle: vi.fn(),
			onread: vi.fn().mockResolvedValue({ status: "ready", data: issue }),
		},
	});
	await tick();
	await vi.waitFor(() => expect(target.querySelector("a strong")).not.toBeNull());
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

it("scrolls to the top when switching to an edition whose detail is pending", async () => {
	const target = document.createElement("main");
	target.scrollTo = vi.fn();
	document.body.append(target);
	let complete!: (response: { status: "ready"; data: KnowledgeDocument }) => void;
	const pending = new Promise<{ status: "ready"; data: KnowledgeDocument }>((resolve) => {
		complete = resolve;
	});
	const personal = {
		...issue,
		id: "personal",
		title: "Personal",
		newspaperEdition: "personal" as const,
	};
	const view = mount(NewspaperView, {
		target,
		props: {
			documents: [issue, personal],
			issues: { developer: issue.id, personal: personal.id },
			loading: false,
			onread: vi
				.fn()
				.mockResolvedValueOnce({ status: "ready", data: issue })
				.mockReturnValueOnce(pending),
			onopenarticle: vi.fn(),
		},
	});
	await vi.waitFor(() => expect(target.querySelector(".copy")).not.toBeNull());
	target.scrollTop = 900;
	target.querySelector<HTMLButtonElement>('[aria-label="翻到每日日报"]')!.click();
	await tick();
	expect(target.querySelector('[aria-label="Loading newspaper"]')).not.toBeNull();
	await vi.waitFor(() =>
		expect(target.scrollTo).toHaveBeenCalledWith({ top: 0, behavior: "instant" }),
	);
	complete({ status: "ready", data: personal });
	await vi.waitFor(() => expect(target.querySelector(".cover h2")?.textContent).toBe("Personal"));
	expect(target.querySelector('[role="status"]')).toBeNull();
	await unmount(view);
	target.remove();
});
