import { expect, it } from "vite-plus/test";
import { mount, unmount } from "svelte";
import InvalidWidget from "../InvalidWidget.svelte";

it("shows only the card configuration as escaped text", async () => {
	const target = document.createElement("div");
	const configuration = '{"kind":"spending","extra":"<img src=x onerror=alert(1)>"}';
	const view = mount(InvalidWidget, {
		target,
		props: { configuration, error: "Unknown field extra" },
	});
	try {
		expect(target.querySelector("pre")?.textContent).toBe(configuration);
		expect(target.querySelector("img")).toBeNull();
		expect(target.querySelector("details")?.open).toBe(false);
		expect(target.querySelector("details")?.textContent).toContain("Unknown field extra");
	} finally {
		await unmount(view);
	}
});
