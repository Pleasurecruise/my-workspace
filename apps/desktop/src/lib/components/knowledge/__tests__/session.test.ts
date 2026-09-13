import { beforeEach, expect, it, vi } from "vite-plus/test";
import type { ChannelView, CommandResponse, KnowledgeDocument } from "../../../consumer";
import { createKnowledgeSession } from "../session.svelte";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("svelte", async (importOriginal) => ({
	...(await importOriginal<typeof import("svelte")>()),
	onMount: vi.fn(),
}));
beforeEach(() => invoke.mockReset());

const document: KnowledgeDocument = {
	id: "article",
	slug: "article",
	title: "Original",
	summary: "",
	tags: [],
	visibility: "private",
	contentHash: "original",
	createdAt: "2026-09-07T00:00:00Z",
	updatedAt: "2026-09-07T00:00:00Z",
	newspaperEdition: null,
	source: "Body",
	html: "<p>Body</p>",
	toc: [],
	stats: { wordCount: 1, readingMinutes: 1 },
};
function page(article: KnowledgeDocument): CommandResponse<ChannelView> {
	return {
		status: "ready",
		data: {
			channel: "knowledge",
			knowledge: [article],
			newspaper: { developer: null, personal: null },
			nextCursor: null,
		},
	};
}
function deferred<T>() {
	let resolve: (value: T) => void = () => {
		throw new Error("Promise not initialized");
	};
	const promise = new Promise<T>((settle) => {
		resolve = settle;
	});
	return { promise, resolve };
}

it.each([false, true])(
	"settles a save after navigation while rejecting a replaced credential session (%s)",
	async (reset) => {
		const session = createKnowledgeSession({ active: false, mainElement: null });
		session.initialize(page(document), session.version);
		const pending = deferred<CommandResponse<KnowledgeDocument>>();
		invoke.mockReturnValue(pending.promise);
		const save = session.updateKnowledge(document.id, {
			title: "Saved",
			summary: "",
			body: "Body",
			tags: [],
			expectedHash: document.contentHash,
		});
		session.leave();
		if (reset) {
			session.reset();
			session.initialize(page({ ...document, title: "New account" }), session.version);
		}
		pending.resolve({ status: "ready", data: { ...document, title: "Saved" } });
		await save;
		expect(session.content?.knowledge[0]?.title).toBe(reset ? "New account" : "Saved");
	},
);

it("rejects an older startup snapshot after a feature refresh", async () => {
	const session = createKnowledgeSession({ active: false, mainElement: null });
	const version = session.version;
	invoke.mockResolvedValue(page({ ...document, title: "Fresh" }));
	await session.refresh();
	session.initialize(page(document), version);
	expect(session.content?.knowledge[0]?.title).toBe("Fresh");
});

it("keeps the settled overview and exposes a failed refresh", async () => {
	const session = createKnowledgeSession({ active: false, mainElement: null });
	session.initialize(page(document), session.version);
	invoke.mockResolvedValue({ status: "failed", message: "Read failed" });
	await session.refresh();
	expect(session.content?.knowledge[0]?.title).toBe("Original");
	expect(session.error).toBe("Read failed");
	expect(session.loading).toBe(false);
});

it("preserves a saved article when an older refresh completes", async () => {
	const session = createKnowledgeSession({ active: false, mainElement: null });
	session.initialize(page(document), session.version);
	const pending = deferred<CommandResponse<ChannelView>>();
	invoke.mockImplementation((command: string) =>
		command === "read_channel"
			? pending.promise
			: Promise.resolve({ status: "ready", data: { ...document, title: "Saved" } }),
	);
	const read = session.refresh();
	await session.updateKnowledge(document.id, {
		title: "Saved",
		summary: "",
		body: "Body",
		tags: [],
		expectedHash: document.contentHash,
	});
	expect(session.content?.knowledge[0]?.title).toBe("Saved");
	pending.resolve(page(document));
	await read;
	expect(session.content?.knowledge[0]?.title).toBe("Saved");
	expect(session.loading).toBe(false);
});

it("does not duplicate a created document already observed by a refresh", async () => {
	const session = createKnowledgeSession({ active: false, mainElement: null });
	const pending = deferred<CommandResponse<KnowledgeDocument>>();
	invoke.mockImplementation((command: string) =>
		command === "read_channel" ? Promise.resolve(page(document)) : pending.promise,
	);
	const write = session.createKnowledge({
		title: document.title,
		summary: "",
		body: "Body",
		tags: [],
	});
	await session.refresh();
	pending.resolve({ status: "ready", data: document });
	await write;
	expect(session.content?.knowledge).toEqual([document]);
});
