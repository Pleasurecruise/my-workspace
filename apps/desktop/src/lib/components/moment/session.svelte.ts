import { invoke } from "@tauri-apps/api/core";
import { onMount, tick } from "svelte";
import type {
	ChannelView,
	CommandResponse,
	PhotoUpload,
	PhotoUpdate,
	PhotoItem,
} from "../../consumer";

export function createMomentTags() {
	let tags = $state<string[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let session = 0;
	let request = 0;
	async function refresh(supersede = false) {
		if (loading && !supersede) return;
		const version = ++request;
		loading = true;
		let response: CommandResponse<string[]>;
		try {
			response = await invoke<CommandResponse<string[]>>("read_moment_tags");
		} catch {
			// Rejected IPC is a transport failure, outside tagged provider responses.
			response = { status: "failed", message: "Could not load tags. Try again." };
		}
		if (version !== request) return;
		loading = false;
		if (response.status === "ready") {
			tags = response.data;
			error = null;
		} else error = response.message;
	}
	return {
		get tags() {
			return tags;
		},
		get error() {
			return error;
		},
		get loading() {
			return loading;
		},
		get session() {
			return session;
		},
		refresh,
		reset() {
			session += 1;
			request += 1;
			tags = [];
			error = null;
			loading = false;
		},
	};
}

export function createMomentSession(context: {
	readonly active: boolean;
	readonly mainElement: HTMLElement | null;
}) {
	let content = $state<Extract<ChannelView, { channel: "moment" }> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let loadingMore = $state(false);
	let request = 0;
	const tags = createMomentTags();
	async function load(cursor: string | null, replace: boolean, showPaginationStatus = false) {
		if (loading) return;
		const version = ++request;
		loading = true;
		loadingMore = cursor !== null && showPaginationStatus;
		const response = await invoke<CommandResponse<ChannelView>>("read_channel", {
			query: {
				channel: "moment",
				cursor,
				search: null,
				tags: [],
				sortByUpdated: false,
				archivedOnly: false,
				favoritesOnly: false,
			},
		});
		if (version !== request) return;
		loading = false;
		loadingMore = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		if (response.data.channel !== "moment") {
			error = "The moment command returned the wrong channel.";
			return;
		}
		const page = response.data;
		if (!replace && content !== null)
			content = { ...page, photos: [...content.photos, ...page.photos], tags: content.tags };
		else content = page;
		error = null;
		await fillViewport(showPaginationStatus);
	}

	async function fillViewport(showPaginationStatus = false) {
		await tick();
		const element = context.mainElement;
		if (
			context.active &&
			element !== null &&
			element.scrollHeight - element.scrollTop - element.clientHeight < 600
		)
			loadMore(showPaginationStatus);
	}

	function loadMore(showPaginationStatus = false) {
		if (!context.active || loading || content === null || content.nextCursor === null) return;
		void load(content.nextCursor, false, showPaginationStatus);
	}

	async function enter(force = false) {
		void tags.refresh();
		if (content === null || force) await load(null, true);
		else {
			error = null;
			await fillViewport();
		}
	}

	function leave() {
		request += 1;
		loading = false;
		loadingMore = false;
	}

	async function refresh() {
		leave();
		void tags.refresh();
		await load(null, true);
	}

	function reset() {
		leave();
		content = null;
		error = null;
		tags.reset();
		void tags.refresh();
	}

	function initialize(response: CommandResponse<ChannelView>, version: number) {
		if (version !== request) return;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		if (response.data.channel !== "moment") {
			error = "The moment command returned the wrong channel.";
			return;
		}
		content = response.data;
		error = null;
	}
	async function createPhoto(input: PhotoUpload, file: File): Promise<CommandResponse<PhotoItem>> {
		const session = tags.session;
		let source: number[];
		try {
			source = Array.from(new Uint8Array(await file.arrayBuffer()));
		} catch {
			return { status: "failed", message: "The photo could not be read." };
		}
		if (session !== tags.session)
			return {
				status: "failed",
				message: "Moment configuration changed. Select the photo again before uploading.",
			};
		const response = await invoke<CommandResponse<PhotoItem>>("create_photo", { input, source });
		if (session !== tags.session)
			return {
				status: "failed",
				message:
					"Moment configuration changed during upload. Reload the gallery to check the result.",
			};
		if (response.status === "ready") {
			void tags.refresh(true);
			if (content !== null) {
				content = {
					...content,
					photos: [response.data, ...content.photos],
					total: content.total + 1,
				};
			}
		}
		return response;
	}

	async function updatePhoto(id: string, input: PhotoUpdate): Promise<CommandResponse<PhotoItem>> {
		const session = tags.session;
		const response = await invoke<CommandResponse<PhotoItem>>("update_photo", { id, input });
		if (session !== tags.session) return response;
		if (response.status === "ready") void tags.refresh(true);
		if (response.status === "ready" && content !== null) {
			content = {
				...content,
				photos: content.photos.map((photo) => (photo.id === id ? response.data : photo)),
			};
		}
		return response;
	}

	async function deletePhoto(id: string): Promise<CommandResponse<string>> {
		const session = tags.session;
		const response = await invoke<CommandResponse<string>>("delete_photo", { id });
		if (session !== tags.session) return response;
		if (response.status === "ready") void tags.refresh(true);
		if (response.status === "ready" && content !== null) {
			const photos = content.photos.filter((photo) => photo.id !== id);
			content = {
				...content,
				photos,
				total: content.total - 1,
			};
		}
		return response;
	}

	onMount(() => {
		void tags.refresh();
		const timer = window.setInterval(() => {
			if (!context.active) return;
			void tags.refresh();
			if (!loading && context.mainElement !== null && context.mainElement.scrollTop < 200)
				void load(null, true);
		}, 60_000);

		return () => {
			leave();
			tags.reset();
			window.clearInterval(timer);
		};
	});
	return {
		get content() {
			return content;
		},
		get error() {
			return error;
		},
		get loading() {
			return loading;
		},
		get loadingMore() {
			return loadingMore;
		},
		get version() {
			return request;
		},
		enter,
		leave,
		refresh,
		reset,
		initialize,
		loadMore,
		tags,
		createPhoto,
		updatePhoto,
		deletePhoto,
	};
}
