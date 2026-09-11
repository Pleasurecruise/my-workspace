import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import RichMarkdownEditor from "../RichMarkdownEditor.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

it("keeps source mode when a compatibility result arrives after the user chooses it", async () => {
	let finish: (value: boolean) => void = () => {};
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
		target.querySelector<HTMLButtonElement>(".mode-switch button:last-child")?.click();
		finish(true);
		await tick();
		expect(target.querySelector("textarea")?.value).toBe("* **Hello**\n* World\n");
		target.querySelector<HTMLButtonElement>(".mode-switch button:first-child")?.click();
		finish(true);
		await vi.waitFor(() => expect(target.querySelector("textarea")).toBeNull());
	} finally {
		await unmount(view);
		target.remove();
	}
});
