import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import RichMarkdownEditor from "../RichMarkdownEditor.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function findElement<T extends Element>(target: ParentNode, selector: string): T {
	const element = target.querySelector<T>(selector);
	if (element === null) throw new Error(`Missing element: ${selector}`);
	return element;
}

it("keeps source mode when a compatibility result arrives after the user chooses it", async () => {
	let finish: (value: boolean) => void = () => {
		throw new Error("Compatibility request has not started");
	};
	invoke.mockImplementation(
		() =>
			new Promise<boolean>((resolve) => {
				finish = resolve;
			}),
	);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(RichMarkdownEditor, { target, props: { value: "* **Hello**\n* World\n" } });
	try {
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("markdown_matches", {
				source: "* **Hello**\n* World\n",
				candidate: expect.any(String),
			}),
		);
		findElement<HTMLButtonElement>(target, ".mode-switch button:last-child").click();
		finish(true);
		await tick();
		expect(findElement<HTMLTextAreaElement>(target, "textarea").value).toBe(
			"* **Hello**\n* World\n",
		);
		findElement<HTMLButtonElement>(target, ".mode-switch button:first-child").click();
		finish(true);
		await vi.waitFor(() => expect(target.querySelector("textarea")).toBeNull());
	} finally {
		await unmount(view);
		target.remove();
	}
});

it("preserves authored Markdown when switching modes without editing", async () => {
	invoke.mockReset();
	invoke.mockResolvedValue(true);
	const source =
		"* **Hello**\n* World\n\n```embed:media\ntype: audio\nsrc: https://example.com/audio.mp3\n```\n";
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(RichMarkdownEditor, { target, props: { value: source } });
	try {
		await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
		await vi.waitFor(() => expect(target.querySelector("textarea")).toBeNull());
		findElement<HTMLButtonElement>(target, ".mode-switch button:last-child").click();
		await tick();
		expect(findElement<HTMLTextAreaElement>(target, "textarea").value).toBe(source);
		findElement<HTMLButtonElement>(target, ".mode-switch button:first-child").click();
		await vi.waitFor(() => expect(target.querySelector("textarea")).toBeNull());
		findElement<HTMLButtonElement>(target, ".mode-switch button:last-child").click();
		await tick();
		expect(findElement<HTMLTextAreaElement>(target, "textarea").value).toBe(source);
	} finally {
		await unmount(view);
		target.remove();
	}
});

it.each(["unsupported", "failed"])(
	"keeps the complete source when compatibility is %s",
	async (result) => {
		invoke.mockReset();
		if (result === "unsupported") invoke.mockResolvedValue(false);
		else invoke.mockRejectedValue(new Error("Unavailable"));
		const source =
			"```embed:article\nhttps://knowledge.you-find.me/articles/reference\n```\n\n![image](https://example.com/image.png)\n";
		const target = document.createElement("div");
		document.body.append(target);
		const view = mount(RichMarkdownEditor, { target, props: { value: source } });
		try {
			await vi.waitFor(() => expect(target.querySelector('[role="status"]')).not.toBeNull());
			expect(findElement<HTMLTextAreaElement>(target, "textarea").value).toBe(source);
			expect(findElement<HTMLElement>(target, ".surface").classList.contains("hidden")).toBe(true);
		} finally {
			await unmount(view);
			target.remove();
		}
	},
);

it("ignores compatibility completion after the source changes", async () => {
	invoke.mockReset();
	let finish: (value: boolean) => void = () => {
		throw new Error("Compatibility request has not started");
	};
	invoke.mockImplementation(
		() =>
			new Promise<boolean>((resolve) => {
				finish = resolve;
			}),
	);
	const target = document.createElement("div");
	document.body.append(target);
	const view = mount(RichMarkdownEditor, { target, props: { value: "Original" } });
	try {
		await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
		const source = target.querySelector("textarea");
		if (source === null) throw new Error("Source mode must remain available during verification");
		const changed = "Changed while checking\n\n```embed:stock\ncode: MSFT\n```";
		source.value = changed;
		source.dispatchEvent(new Event("input", { bubbles: true }));
		await tick();
		finish(true);
		await tick();
		expect(findElement<HTMLTextAreaElement>(target, "textarea").value).toBe(changed);
		expect(findElement<HTMLElement>(target, ".surface").classList.contains("hidden")).toBe(true);
	} finally {
		await unmount(view);
		target.remove();
	}
});
