import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import KnowledgeToc from "../KnowledgeToc.svelte";

it.each([false, true])(
	"scrolls the current reader and keeps pointer selection alive after null focusout (reduced motion: %s)",
	async (reduced) => {
		vi.stubGlobal("matchMedia", () => ({ matches: reduced }));
		const main = document.createElement("main");
		const controls = document.createElement("div");
		const content = document.createElement("article");
		content.innerHTML = '<h2 id="section" style="scroll-margin-top:64px">Section</h2>';
		main.append(controls, content);
		document.body.append(main);
		const heading = content.querySelector("h2")!;
		main.scrollTop = 100;
		main.scrollTo = vi.fn();
		vi.spyOn(main, "getBoundingClientRect").mockReturnValue({ top: 20 } as DOMRect);
		vi.spyOn(heading, "getBoundingClientRect").mockReturnValue({ top: 620 } as DOMRect);
		const view = mount(KnowledgeToc, {
			target: controls,
			props: { entries: [{ id: "section", depth: 2, text: "Section" }], content },
		});
		await tick();
		const panel = controls.querySelector("details")!;
		panel.open = true;
		panel.dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: null }));
		expect(panel.open).toBe(true);
		controls.querySelector("button")!.click();
		expect(main.scrollTo).toHaveBeenCalledWith({
			top: 636,
			behavior: reduced ? "instant" : "smooth",
		});
		expect(panel.open).toBe(false);
		panel.open = true;
		document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
		expect(panel.open).toBe(false);
		await unmount(view);
		main.remove();
		vi.unstubAllGlobals();
	},
);
