import { beforeEach, expect, it, vi } from "vite-plus/test";
import type { ChannelView, CommandResponse, PhotoItem } from "../../../consumer";
import { createMomentSession } from "../session.svelte";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("svelte", async (importOriginal) => ({
	...(await importOriginal<typeof import("svelte")>()),
	onMount: vi.fn(),
}));
beforeEach(() => invoke.mockReset());
function deferred<T>() {
	let resolve: (value: T) => void = () => {
		throw new Error("Promise not initialized");
	};
	const promise = new Promise<T>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}
const original: PhotoItem = {
	id: "photo",
	url: "https://example.test/photo",
	thumbnailUrl: "https://example.test/thumb",
	r2Key: "img/photo.png",
	thumbnailR2Key: "img/thumb.jpg",
	thumbHash: null,
	title: "Original",
	width: 10,
	height: 10,
	aspectRatio: 1,
	tags: [],
	date: null,
	description: null,
	size: null,
	format: null,
	geo: null,
};
function page(photos: PhotoItem[]): CommandResponse<ChannelView> {
	return { status: "ready", data: { channel: "moment", photos, total: photos.length } };
}
it.each(["create", "update", "delete"])(
	"preserves %s after a pre-write gallery read settles",
	async (operation) => {
		const session = createMomentSession({ active: false, mainElement: null });
		session.initialize(page([original]), session.version);
		const pending = deferred<CommandResponse<ChannelView>>();
		const saved = {
			...original,
			id: operation === "create" ? "created" : original.id,
			title: "Saved",
		};
		invoke.mockImplementation((command: string) => {
			if (command === "read_channel") return pending.promise;
			if (command === "read_moment_tags") return Promise.resolve({ status: "ready", data: [] });
			return Promise.resolve({
				status: "ready",
				data: operation === "delete" ? original.id : saved,
			});
		});
		const read = session.refresh();
		if (operation === "create")
			await session.createPhoto(
				{ title: "Saved", description: null, tags: [], date: null, geo: null },
				new File([new Uint8Array([1])], "photo.png"),
			);
		else if (operation === "update")
			await session.updatePhoto(original.id, { title: "Saved", description: "", tags: [] });
		else await session.deletePhoto(original.id);
		const expected =
			operation === "delete" ? [] : operation === "update" ? [saved] : [saved, original];
		expect(session.content?.photos).toEqual(expected);
		pending.resolve(page([original]));
		await read;
		expect(session.content?.photos).toEqual(expected);
		expect(session.content?.total).toBe(expected.length);
		expect(session.loading).toBe(false);
	},
);
it("rejects a write from a replaced credential session", async () => {
	const session = createMomentSession({ active: false, mainElement: null });
	session.initialize(page([original]), session.version);
	const pending = deferred<CommandResponse<string>>();
	invoke.mockImplementation((command: string) =>
		command === "delete_photo" ? pending.promise : Promise.resolve({ status: "ready", data: [] }),
	);
	const write = session.deletePhoto(original.id);
	session.reset();
	session.initialize(page([{ ...original, title: "New account" }]), session.version);
	pending.resolve({ status: "ready", data: original.id });
	await write;
	expect(session.content?.photos[0]?.title).toBe("New account");
});

it.each(["create", "delete"])(
	"keeps gallery IDs and total consistent when a refresh already observes %s",
	async (operation) => {
		const session = createMomentSession({ active: false, mainElement: null });
		session.initialize(page(operation === "create" ? [] : [original]), session.version);
		const pending = deferred<CommandResponse<PhotoItem | string>>();
		invoke.mockImplementation((command: string) => {
			if (command === "read_channel")
				return Promise.resolve(page(operation === "create" ? [original] : []));
			if (command === "read_moment_tags") return Promise.resolve({ status: "ready", data: [] });
			return pending.promise;
		});
		const write =
			operation === "create"
				? session.createPhoto(
						{ title: "Original", description: null, tags: [], date: null, geo: null },
						new File([new Uint8Array([1])], "photo.png"),
					)
				: session.deletePhoto(original.id);
		await vi.waitFor(() =>
			expect(invoke).toHaveBeenCalledWith(
				operation === "create" ? "create_photo" : "delete_photo",
				expect.anything(),
			),
		);
		await session.refresh();
		pending.resolve({ status: "ready", data: operation === "create" ? original : original.id });
		await write;
		expect(session.content?.photos).toEqual(operation === "create" ? [original] : []);
		expect(session.content?.total).toBe(operation === "create" ? 1 : 0);
	},
);
