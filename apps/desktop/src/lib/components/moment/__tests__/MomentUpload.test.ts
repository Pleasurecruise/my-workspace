import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import MomentUpload from "../MomentUpload.svelte";
import type { CommandResponse, PhotoMetadata } from "../../../consumer";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const metadata: CommandResponse<PhotoMetadata> = {
	status: "ready",
	data: { capturedAt: "2026-09-18T12:34:56+08:00", geo: { lat: -31.2, lng: 121.5 } },
};
const views: ReturnType<typeof mount>[] = [];
beforeEach(() => {
	invoke.mockReset().mockResolvedValue(metadata);
	vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:photo");
	vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
});
afterEach(async () => {
	for (const view of views.splice(0)) await unmount(view);
	document.body.replaceChildren();
	vi.restoreAllMocks();
});
function setup() {
	const target = document.createElement("div");
	document.body.append(target);
	const onupload = vi
		.fn()
		.mockResolvedValue({ status: "failed", message: "Test: no remote upload" });
	views.push(
		mount(MomentUpload, { target, props: { onupload, onuploaded: vi.fn(), onclose: vi.fn() } }),
	);
	return { target, onupload };
}
function requireInput(target: HTMLElement, selector: string) {
	const input = target.querySelector(selector);
	if (!(input instanceof HTMLInputElement)) throw new Error(`Missing input: ${selector}`);
	return input;
}
async function select(target: HTMLElement, name = "photo.heic") {
	const file = new File(["synthetic photo"], name, { type: "image/heic" });
	const input = requireInput(target, 'input[type="file"]');
	Object.defineProperty(input, "files", { configurable: true, value: { item: () => file } });
	input.dispatchEvent(new Event("change", { bubbles: true }));
	await tick();
	await vi.waitFor(() =>
		expect(invoke).toHaveBeenCalledWith("read_photo_metadata", expect.anything()),
	);
}
function fill(input: HTMLInputElement, value: string) {
	input.value = value;
	input.dispatchEvent(new Event("input", { bubbles: true }));
}
function publish(target: HTMLElement) {
	const button = Array.from(target.querySelectorAll("button")).find(
		(button) => button.textContent === "Publish",
	);
	if (button === undefined) throw new Error("Missing Publish button");
	button.click();
}
it("prefills HEIC GPS and capture time before publication and retains the camera offset", async () => {
	const { target, onupload } = setup();
	await select(target);
	await vi.waitFor(() =>
		expect(target.querySelector<HTMLInputElement>('input[type="datetime-local"]')?.value).toBe(
			"2026-09-18T12:34:56",
		),
	);
	const coordinates = target.querySelectorAll<HTMLInputElement>('input[type="number"]');
	expect(Number(coordinates[0]?.value)).toBe(-31.2);
	expect(Number(coordinates[1]?.value)).toBe(121.5);
	expect(onupload).not.toHaveBeenCalled();
	publish(target);
	await vi.waitFor(() =>
		expect(onupload).toHaveBeenCalledWith(
			expect.objectContaining({
				date: "2026-09-18T12:34:56+08:00",
				geo: { lat: -31.2, lng: 121.5 },
			}),
			expect.any(File),
		),
	);
});
it("preserves manual input during the read and accepts numeric input events", async () => {
	let resolve = (_value: CommandResponse<PhotoMetadata>) => {};
	invoke.mockReturnValue(
		new Promise<CommandResponse<PhotoMetadata>>((settle) => (resolve = settle)),
	);
	const { target, onupload } = setup();
	await select(target);
	fill(requireInput(target, 'input[type="datetime-local"]'), "2026-09-17T10:20");
	fill(requireInput(target, 'input[type="number"][min="-90"]'), "51.5");
	fill(requireInput(target, 'input[type="number"][min="-180"]'), "-0.12");
	publish(target);
	expect(onupload).not.toHaveBeenCalled();
	resolve(metadata);
	await vi.waitFor(() =>
		expect(target.querySelector('[role="status"]')?.textContent).toContain("loaded"),
	);
	publish(target);
	await vi.waitFor(() =>
		expect(onupload).toHaveBeenCalledWith(
			expect.objectContaining({
				date: new Date("2026-09-17T10:20").toISOString(),
				geo: { lat: 51.5, lng: -0.12 },
			}),
			expect.any(File),
		),
	);
});
it("submits explicitly cleared metadata as absent", async () => {
	const { target, onupload } = setup();
	await select(target);
	await vi.waitFor(() =>
		expect(target.querySelector('[role="status"]')?.textContent).toContain("loaded"),
	);
	fill(requireInput(target, 'input[type="datetime-local"]'), "");
	for (const input of target.querySelectorAll<HTMLInputElement>('input[type="number"]'))
		fill(input, "");
	await tick();
	publish(target);
	await vi.waitFor(() =>
		expect(onupload).toHaveBeenCalledWith(
			expect.objectContaining({ date: null, geo: null }),
			expect.any(File),
		),
	);
});
it("discards the removed photo's late metadata after selecting another photo", async () => {
	let resolve = (_value: CommandResponse<PhotoMetadata>) => {};
	invoke.mockReturnValueOnce(
		new Promise<CommandResponse<PhotoMetadata>>((settle) => (resolve = settle)),
	);
	const { target } = setup();
	await select(target, "old.heic");
	target.querySelector<HTMLButtonElement>('[aria-label="Remove selected image"]')?.click();
	await tick();
	invoke.mockResolvedValueOnce({ status: "ready", data: { capturedAt: null, geo: null } });
	await select(target, "new.heic");
	await vi.waitFor(() =>
		expect(target.querySelector('[role="status"]')?.textContent).toContain("No capture"),
	);
	resolve(metadata);
	await tick();
	expect(target.querySelector<HTMLInputElement>('input[type="datetime-local"]')?.value).toBe("");
	expect(target.querySelector<HTMLInputElement>('input[type="number"]')?.value).toBe("");
});
it("keeps the form usable when metadata reading fails", async () => {
	invoke.mockRejectedValueOnce(new Error("IPC unavailable"));
	const { target, onupload } = setup();
	await select(target);
	await vi.waitFor(() =>
		expect(target.querySelector('[role="alert"]')?.textContent).toContain("Could not read"),
	);
	publish(target);
	await vi.waitFor(() => expect(onupload).toHaveBeenCalled());
});

it("uses an edited time even when it returns to the camera's original wall clock", async () => {
	const { target, onupload } = setup();
	await select(target);
	await vi.waitFor(() =>
		expect(target.querySelector('[role="status"]')?.textContent).toContain("loaded"),
	);
	const date = requireInput(target, 'input[type="datetime-local"]');
	fill(date, "2026-09-17T12:34:56");
	fill(date, "2026-09-18T12:34:56");
	await tick();
	publish(target);
	await vi.waitFor(() =>
		expect(onupload).toHaveBeenCalledWith(
			expect.objectContaining({ date: new Date("2026-09-18T12:34:56").toISOString() }),
			expect.any(File),
		),
	);
});

it("discards a pending file read when the upload form closes", async () => {
	const { target } = setup();
	let release = (_bytes: ArrayBuffer) => {};
	const file = new File(["photo"], "photo.heic", { type: "image/heic" });
	vi.spyOn(file, "arrayBuffer").mockReturnValue(
		new Promise<ArrayBuffer>((resolve) => (release = resolve)),
	);
	const input = requireInput(target, 'input[type="file"]');
	Object.defineProperty(input, "files", { configurable: true, value: { item: () => file } });
	input.dispatchEvent(new Event("change", { bubbles: true }));
	await tick();
	const view = views.pop();
	if (view === undefined) throw new Error("Missing mounted upload form");
	await unmount(view);
	release(new ArrayBuffer(4));
	await tick();
	expect(invoke).not.toHaveBeenCalled();
});
