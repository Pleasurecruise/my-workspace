import { SvelteSet } from "svelte/reactivity";
import { invoke } from "@tauri-apps/api/core";
import { onMount, tick } from "svelte";
import type {
	ChannelView,
	CommandResponse,
	MemoView,
	MemoUpdate,
	PublishedPost,
	MemoTagCount,
} from "../../consumer";

type MemoDisplay = "active" | "favorites" | "archived";

export function createMemosTags() {
	let tags = $state<MemoTagCount[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let session = 0;
	let request = 0;
	async function refresh(supersede = false) {
		if (loading && !supersede) return;
		const version = ++request;
		loading = true;
		let response: CommandResponse<MemoTagCount[]>;
		try {
			response = await invoke<CommandResponse<MemoTagCount[]>>("read_memo_tags");
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

export function createMemosSession(context: {
	readonly active: boolean;
	readonly mainElement: HTMLElement | null;
}) {
	let content = $state<Extract<ChannelView, { channel: "memos" }> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let loadingMore = $state(false);
	let request = 0;
	const tags = createMemosTags();
	let memoDisplay = $state<MemoDisplay>("active");
	let memoFilterRequest = 0;
	let memoSearch = "";
	let memoTags: string[] = [];
	let memoSortByUpdated = false;
	let filtersChanged = false;
	async function load(cursor: string | null, replace: boolean, showPaginationStatus = false) {
		if (loading) return;
		const version = ++request;
		loading = true;
		loadingMore = cursor !== null && showPaginationStatus;
		const response = await invoke<CommandResponse<ChannelView>>("read_channel", {
			query: {
				channel: "memos",
				cursor,
				search: memoSearch === "" ? null : memoSearch,
				tags: memoTags,
				sortByUpdated: memoSortByUpdated,
				archivedOnly: memoDisplay === "archived",
				favoritesOnly: memoDisplay === "favorites",
			},
		});
		if (version !== request) return;
		loading = false;
		loadingMore = false;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		if (response.data.channel !== "memos") {
			error = "The memos command returned the wrong channel.";
			return;
		}
		const page = response.data;
		if (replace && content !== null && !filtersChanged) {
			const tail = content.memos.slice(25);
			const refreshedIds = new SvelteSet(page.memos.map((memo) => memo.id));
			content = {
				...page,
				memos: [...page.memos, ...tail.filter((memo) => !refreshedIds.has(memo.id))],
				nextCursor: tail.length > 0 ? content.nextCursor : page.nextCursor,
			};
		} else if (!replace && content !== null) {
			content = { ...page, memos: [...content.memos, ...page.memos] };
		} else content = page;
		if (replace) filtersChanged = false;
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
		if (
			!context.active ||
			loading ||
			filtersChanged ||
			content === null ||
			content.nextCursor === null
		)
			return;
		void load(content.nextCursor, false, showPaginationStatus);
	}

	async function enter(force = false) {
		void tags.refresh();
		if (content === null || force || filtersChanged) await load(null, true);
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
		if (response.data.channel !== "memos") {
			error = "The memos command returned the wrong channel.";
			return;
		}
		content = response.data;
		error = null;
	}
	async function filterMemos(
		search: string,
		tags: string[],
		sortByUpdated: boolean,
		display: MemoDisplay,
	): Promise<string | null> {
		const version = ++memoFilterRequest;
		const requestVersion = ++request;
		filtersChanged = true;
		memoDisplay = display;
		memoSearch = search;
		memoTags = tags;
		memoSortByUpdated = sortByUpdated;
		loading = true;
		loadingMore = false;
		const response = await invoke<CommandResponse<ChannelView>>("read_channel", {
			query: {
				channel: "memos",
				cursor: null,
				search: search === "" ? null : search,
				tags,
				sortByUpdated,
				archivedOnly: display === "archived",
				favoritesOnly: display === "favorites",
			},
		});
		if (version !== memoFilterRequest || requestVersion !== request || !context.active) return null;
		loading = false;
		loadingMore = false;
		if (response.status === "failed") {
			return response.message;
		}
		if (response.data.channel !== "memos") return "The memo command returned the wrong channel.";
		const page = response.data;
		content = page;
		filtersChanged = false;
		error = null;
		await tick();
		if (
			context.mainElement !== null &&
			context.mainElement.scrollHeight -
				context.mainElement.scrollTop -
				context.mainElement.clientHeight <
				600
		)
			loadMore();
		return null;
	}

	async function revealMemo(id: string): Promise<boolean> {
		if (!context.active || filtersChanged || content === null) return false;
		if (content.memos.some((memo) => memo.id === id)) return true;
		if (loading) return false;

		while (content.nextCursor !== null) {
			const cursor = content.nextCursor;
			await load(cursor, false);
			if (content === null || content.channel !== "memos") return false;
			if (content.memos.some((memo) => memo.id === id)) return true;
			if (content.nextCursor === cursor) return false;
		}
		return false;
	}

	function settleWrite() {
		leave();
		void tags.refresh(true);
		// Filter membership and ordering are owned by the API. Never combine a
		// cursor from the old filter with newly selected query parameters.
		if (
			filtersChanged ||
			memoSearch !== "" ||
			memoTags.length > 0 ||
			memoSortByUpdated ||
			memoDisplay !== "active"
		) {
			filtersChanged = true;
			if (context.active) void load(null, true);
		}
	}

	async function createMemo(
		markdown: string,
		visibility: "public" | "private",
	): Promise<CommandResponse<MemoView>> {
		const session = tags.session;
		const response = await invoke<CommandResponse<MemoView>>("create_memo", {
			content: markdown,
			visibility,
		});
		if (session !== tags.session) return response;
		if (response.status === "ready") settleWrite();
		if (response.status === "ready" && content !== null && !filtersChanged) {
			content = {
				...content,
				memos: [response.data, ...content.memos.filter((item) => item.id !== response.data.id)],
			};
		}
		return response;
	}

	async function importXMemo(
		url: string,
		visibility: "public" | "private",
	): Promise<CommandResponse<MemoView>> {
		const session = tags.session;
		const response = await invoke<CommandResponse<MemoView>>("import_x_memo", { url, visibility });
		if (session !== tags.session) return response;
		if (response.status === "ready") settleWrite();
		if (response.status === "ready" && content !== null && !filtersChanged) {
			content = {
				...content,
				memos: [response.data, ...content.memos.filter((item) => item.id !== response.data.id)],
			};
		}
		return response;
	}

	async function updateMemo(id: string, input: MemoUpdate): Promise<CommandResponse<MemoView>> {
		const session = tags.session;
		const response = await invoke<CommandResponse<MemoView>>("update_memo", {
			id,
			input,
		});
		if (session !== tags.session) return response;
		if (response.status === "ready") settleWrite();
		if (response.status === "ready" && content !== null && !filtersChanged) {
			content = {
				...content,
				memos: content.memos.map((memo) => (memo.id === id ? response.data : memo)),
			};
		}
		return response;
	}

	async function deleteMemo(id: string): Promise<CommandResponse<string>> {
		const session = tags.session;
		const response = await invoke<CommandResponse<string>>("delete_memo", { id });
		if (session !== tags.session) return response;
		if (response.status === "ready") settleWrite();
		if (response.status === "ready" && content !== null) {
			content = { ...content, memos: content.memos.filter((memo) => memo.id !== id) };
		}
		return response;
	}

	async function publishMemoToTelegram(memo: MemoView): Promise<CommandResponse<PublishedPost>> {
		return invoke<CommandResponse<PublishedPost>>("publish_telegram", { id: memo.id });
	}

	async function publishMemoToX(memo: MemoView): Promise<CommandResponse<PublishedPost>> {
		return invoke<CommandResponse<PublishedPost>>("publish_x", { id: memo.id });
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
		get memoDisplay() {
			return memoDisplay;
		},
		set memoDisplay(value: MemoDisplay) {
			if (memoDisplay === value) return;
			leave();
			filtersChanged = true;
			memoDisplay = value;
		},
		createMemo,
		importXMemo,
		updateMemo,
		deleteMemo,
		publishMemoToTelegram,
		publishMemoToX,
		filterMemos,
		revealMemo,
	};
}
