import { beforeEach, expect, it, vi } from "vite-plus/test";
import { openArticleLinks } from "../links";

const opener = vi.hoisted(() => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => opener);
beforeEach(() => {
	opener.openUrl.mockReset();
});

it("keeps the reader in place and reports browser failures", async () => {
	opener.openUrl.mockRejectedValue(new Error("Unavailable"));
	const node = document.createElement("article");
	node.innerHTML = '<a href="https://example.com">Story</a>';
	const report = vi.fn();
	const action = openArticleLinks(node, report);
	const link = node.querySelector("a");
	if (!link) throw new Error("Missing link");
	const event = new MouseEvent("click", { bubbles: true, cancelable: true });
	link.dispatchEvent(event);
	expect(event.defaultPrevented).toBe(true);
	await vi.waitFor(() =>
		expect(report).toHaveBeenLastCalledWith(
			"Could not open the link in your browser. Please try again.",
		),
	);
	action.destroy();
});

it("blocks unsupported and relative navigation while leaving fragment links local", () => {
	const node = document.createElement("article");
	const report = vi.fn();
	const action = openArticleLinks(node, report);
	for (const href of ["javascript:alert(1)", "file:///tmp/example", "/another-page"]) {
		node.innerHTML = `<a href="${href}">Story</a>`;
		const link = node.querySelector("a");
		if (!link) throw new Error("Missing link");
		const event = new MouseEvent("click", { bubbles: true, cancelable: true });
		link.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(true);
	}
	node.innerHTML = '<a href="#section">Section</a>';
	const fragment = node.querySelector("a");
	if (!fragment) throw new Error("Missing fragment");
	const event = new MouseEvent("click", { bubbles: true, cancelable: true });
	fragment.dispatchEvent(event);
	expect(event.defaultPrevented).toBe(false);
	expect(opener.openUrl).not.toHaveBeenCalled();
	action.destroy();
});

it.each(["next click", "destroy"])("ignores late failures after %s", async (transition) => {
	let reject: (error: Error) => void = () => {
		throw new Error("Promise not initialized");
	};
	const pending = new Promise<void>((_resolve, fail) => {
		reject = fail;
	});
	opener.openUrl.mockReturnValueOnce(pending).mockResolvedValue(undefined);
	const node = document.createElement("article");
	node.innerHTML = '<a href="https://example.com">Story</a>';
	const report = vi.fn();
	const action = openArticleLinks(node, report);
	const link = node.querySelector("a");
	if (!link) throw new Error("Missing link");
	link.click();
	if (transition === "next click") link.click();
	else action.destroy();
	report.mockClear();
	reject(new Error("Late failure"));
	await pending.catch(() => {});
	expect(report).not.toHaveBeenCalled();
	action.destroy();
});
