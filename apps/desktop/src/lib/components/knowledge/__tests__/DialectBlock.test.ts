import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import { writable } from "svelte/store";
import type { CommandResponse } from "../../../consumer";
import DialectBlock from "../DialectBlock.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

it("discards stale compilation, retains editable source and retries failures", async () => {
	let finish: (value: CommandResponse<string>) => void = () => {};
	invoke.mockImplementationOnce(
		() =>
			new Promise((resolve) => {
				finish = resolve;
			}),
	);
	const source = writable("old source");
	const content = document.createElement("pre");
	content.textContent = "source owned by ProseMirror";
	const target = document.createElement("div");
	document.body.append(target);
	const onEdit = vi.fn();
	const component = mount(DialectBlock, {
		target,
		props: { source, content, context: () => "whole document", onEdit },
	});
	try {
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith("preview_knowledge", {
				source: "old source",
				context: "whole document",
			}),
		);
		invoke.mockResolvedValue({ status: "failed", message: "Invalid annotation" });
		source.set("new source");
		await tick();
		finish({ status: "ready", data: "<p>Stale result</p>" });
		await vi.waitFor(() => expect(target.textContent).toContain("Invalid annotation"));
		expect(target.textContent).not.toContain("Stale result");
		const [toggle, refresh] = target.querySelectorAll("button");
		if (!toggle || !refresh) throw new Error("Block controls missing");
		toggle.click();
		await tick();
		await vi.waitFor(() => expect(onEdit).toHaveBeenLastCalledWith(true));
		expect(content.parentElement?.hidden).toBe(false);
		toggle.click();
		await tick();
		expect(onEdit).toHaveBeenLastCalledWith(false);
		expect(content.parentElement?.hidden).toBe(true);
		invoke.mockResolvedValue({ status: "ready", data: "<p>Compiled block</p>" });
		refresh.click();
		await vi.waitFor(() => expect(target.textContent).toContain("Compiled block"));
		expect(content.textContent).toBe("source owned by ProseMirror");
	} finally {
		await unmount(component);
		target.remove();
	}
});
