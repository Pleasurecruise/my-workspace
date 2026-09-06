import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { fromStore, writable } from "svelte/store";
import type { CommandResponse, MemoView, MemoTagCount } from "../../../consumer";
import MemosView from "../MemosView.svelte";

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (!element) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("retains a composer draft across navigation and keeps edits made during a save", async () => {
	let resolvePending: (response: CommandResponse<MemoView>) => void = () => {
		throw new Error("Promise not initialized");
	};
	const pending = new Promise<CommandResponse<MemoView>>((resolve) => {
		resolvePending = resolve;
	});
	const props = {
		memos: [],
		tags: [],
		display: "active" as const,
		onfilter: vi.fn().mockResolvedValue(null),
		onopenmemo: vi.fn(),
		oncreate: vi.fn(() => pending),
		onimportx: vi.fn(),
		onupdate: vi.fn(),
		ondelete: vi.fn(),
		onpublishtelegram: vi.fn(),
		onpublishx: vi.fn(),
	};
	const target = document.createElement("div");
	document.body.append(target);
	let view = mount(MemosView, { target, props });
	await tick();
	let input = findElement<HTMLTextAreaElement>(target, "textarea");
	input.value = "Submitted draft";
	input.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	findElement<HTMLButtonElement>(target, ".composer-toolbar button:last-child").click();
	expect(props.oncreate).toHaveBeenCalledWith("Submitted draft", "private");
	input.value = "Draft with later edits";
	input.dispatchEvent(new Event("input", { bubbles: true }));
	await tick();
	await unmount(view);
	view = mount(MemosView, { target, props });
	await tick();
	input = findElement<HTMLTextAreaElement>(target, "textarea");
	expect(input.value).toBe("Draft with later edits");
	expect(
		findElement<HTMLButtonElement>(target, ".composer-toolbar button:last-child").disabled,
	).toBe(true);
	resolvePending({
		status: "ready",
		data: {
			id: "memo-1",
			r2Key: "memo-1.md",
			content: "Submitted draft",
			html: "<p>Submitted draft</p>",
			tags: [],
			createdAt: "2026-09-05",
			updatedAt: "2026-09-05",
			visibility: "private",
			pinned: false,
			favorite: false,
			archived: false,
			metadataComplete: true,
		},
	});
	await vi.waitFor(() =>
		expect(
			findElement<HTMLButtonElement>(target, ".composer-toolbar button:last-child").disabled,
		).toBe(false),
	);
	expect(input.value).toBe("Draft with later edits");
	await unmount(view);
	target.remove();
});

it.each(["active", "favorites"] as const)(
	"edits %s memos and retains failed saves",
	async (display) => {
		const memo: MemoView = {
			id: "public-memo",
			r2Key: "public-memo.md",
			content: "Original",
			html: "<p>Original</p>",
			tags: [],
			createdAt: "2026-09-05",
			updatedAt: "2026-09-05",
			visibility: "public",
			pinned: false,
			favorite: display === "favorites",
			archived: false,
			metadataComplete: true,
		};
		const props = {
			memos: [memo],
			tags: [],
			display,
			onfilter: vi.fn().mockResolvedValue(null),
			onopenmemo: vi.fn(),
			oncreate: vi.fn(),
			onimportx: vi.fn(),
			onupdate: vi
				.fn()
				.mockResolvedValue({ status: "ready", data: memo })
				.mockResolvedValueOnce({ status: "failed", message: "Save unavailable" })
				.mockResolvedValueOnce({ status: "ready", data: { ...memo, content: "Revised" } }),
			ondelete: vi.fn().mockResolvedValue({ status: "ready", data: memo.id }),
			onpublishtelegram: vi.fn(),
			onpublishx: vi.fn(),
		};
		const target = document.createElement("div");
		document.body.append(target);
		let view = mount(MemosView, { target, props });
		await tick();
		const edit = Array.from(
			target.querySelectorAll<HTMLButtonElement>("article footer button"),
		).find((button) => button.textContent?.trim() === "Edit");
		if (!edit) throw new Error("Missing Edit action");
		edit.click();
		await tick();
		await unmount(view);
		view = mount(MemosView, { target, props });
		await tick();
		expect(findElement<HTMLTextAreaElement>(target, ".inline-editor textarea").value).toBe(
			"Original",
		);
		findElement<HTMLButtonElement>(target, "article footer button:last-child").click();
		await tick();
		expect(props.onupdate).not.toHaveBeenCalled();
		expect(target.querySelector(".inline-editor")).toBeNull();
		const reopen = Array.from(
			target.querySelectorAll<HTMLButtonElement>("article footer button"),
		).find((button) => button.textContent?.trim() === "Edit");
		if (!reopen) throw new Error("Missing Edit action");
		reopen.click();
		await tick();
		const input = findElement<HTMLTextAreaElement>(target, ".inline-editor textarea");
		input.value = "Revised";
		input.dispatchEvent(new Event("input", { bubbles: true }));
		await tick();
		findElement<HTMLButtonElement>(target, "article footer button:last-child").click();
		await vi.waitFor(() => expect(target.textContent).toContain("Save unavailable"));
		await unmount(view);
		view = mount(MemosView, { target, props });
		await tick();
		expect(findElement<HTMLTextAreaElement>(target, ".inline-editor textarea").value).toBe(
			"Revised",
		);
		findElement<HTMLButtonElement>(target, "article footer button:last-child").click();
		await vi.waitFor(() => expect(target.querySelector(".inline-editor")).toBeNull());
		expect(props.onupdate).toHaveBeenCalledTimes(2);
		expect(props.onupdate).toHaveBeenLastCalledWith("public-memo", {
			content: "Revised",
			visibility: "public",
		});
		if (display === "favorites") {
			findElement<HTMLButtonElement>(target, '[title="Unfavorite memo"]').click();
			await vi.waitFor(() =>
				expect(props.onupdate).toHaveBeenLastCalledWith(memo.id, { favorite: false }),
			);
			findElement<HTMLButtonElement>(target, "article footer button:last-child").click();
			await tick();
			expect(target.textContent).toContain("Permanently delete?");
			findElement<HTMLButtonElement>(target, "article footer button:last-child").click();
			await vi.waitFor(() => expect(props.ondelete).toHaveBeenCalledWith(memo.id));
		}
		await unmount(view);
		target.remove();
	},
);

it("can remove a selected tag after it disappears from the refreshed index", async () => {
	const tags = writable<MemoTagCount[]>([{ name: "old", count: 1 }]);
	const snapshot = fromStore(tags);
	const onfilter = vi.fn().mockResolvedValue(null);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(MemosView, {
		target,
		props: {
			memos: [],
			get tags() {
				return snapshot.current;
			},
			display: "active",
			onfilter,
			onopenmemo: vi.fn(),
			oncreate: vi.fn(),
			onimportx: vi.fn(),
			onupdate: vi.fn(),
			ondelete: vi.fn(),
			onpublishtelegram: vi.fn(),
			onpublishx: vi.fn(),
		},
	});
	await tick();
	const tag = target.querySelector<HTMLButtonElement>('[aria-label="Memo tags"] button');
	if (!tag) throw new Error("Tag missing");
	tag.click();
	await vi.waitFor(() => expect(onfilter).toHaveBeenLastCalledWith("", ["old"], false, "active"));
	tags.set([]);
	await tick();
	const remove = target.querySelector<HTMLButtonElement>('[aria-label="Remove old filter"]');
	if (!remove) throw new Error("Selected tag cannot be removed");
	remove.click();
	await vi.waitFor(() => expect(onfilter).toHaveBeenLastCalledWith("", [], false, "active"));
	await unmount(view);
	target.remove();
});

it("shows tag edge controls only for hidden tags and scrolls to both ends", async () => {
	let resize = () => {};
	vi.stubGlobal(
		"ResizeObserver",
		class {
			constructor(callback: ResizeObserverCallback) {
				resize = () => callback([], this);
			}
			observe() {}
			unobserve() {}
			disconnect() {}
		},
	);
	const tags = writable<MemoTagCount[]>([{ name: "tag", count: 1 }]);
	const snapshot = fromStore(tags);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(MemosView, {
		target,
		props: {
			memos: [],
			get tags() {
				return snapshot.current;
			},
			display: "active",
			onfilter: vi.fn().mockResolvedValue(null),
			onopenmemo: vi.fn(),
			oncreate: vi.fn(),
			onimportx: vi.fn(),
			onupdate: vi.fn(),
			ondelete: vi.fn(),
			onpublishtelegram: vi.fn(),
			onpublishx: vi.fn(),
		},
	});
	await tick();
	const strip = findElement<HTMLDivElement>(target, '[aria-label="Memo tags"]');
	const scroll = vi.spyOn(strip, "scrollTo").mockImplementation(() => {});
	Object.defineProperties(strip, {
		clientWidth: { configurable: true, value: 200 },
		scrollWidth: { configurable: true, value: 800 },
	});
	tags.set([{ name: "updated", count: 2 }]);
	await tick();
	expect(target.querySelector('[aria-label="Scroll tags to start"]')).toBeNull();
	findElement<HTMLButtonElement>(target, '[aria-label="Scroll tags to end"]').click();
	expect(scroll).toHaveBeenLastCalledWith(expect.objectContaining({ left: 800 }));
	strip.scrollLeft = 600;
	strip.dispatchEvent(new Event("scroll"));
	await tick();
	expect(target.querySelector('[aria-label="Scroll tags to end"]')).toBeNull();
	findElement<HTMLButtonElement>(target, '[aria-label="Scroll tags to start"]').click();
	expect(scroll).toHaveBeenLastCalledWith(expect.objectContaining({ left: 0 }));
	strip.scrollLeft = 0;
	Object.defineProperty(strip, "scrollWidth", { value: 200 });
	resize();
	await tick();
	expect(target.querySelector('[aria-label="Scroll tags to start"]')).toBeNull();
	expect(target.querySelector('[aria-label="Scroll tags to end"]')).toBeNull();
	await unmount(view);
	target.remove();
	vi.unstubAllGlobals();
});
