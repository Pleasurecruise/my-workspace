import { SvelteDate } from "svelte/reactivity";
import { invoke } from "@tauri-apps/api/core";
import { onMount, tick } from "svelte";
import type {
	ChannelView,
	CommandResponse,
	KnowledgeDraft,
	KnowledgeUpdate,
	KnowledgeDocument,
} from "../../consumer";

export function createKnowledgeSession(context: {
	readonly active: boolean;
	readonly mainElement: HTMLElement | null;
}) {
	let content = $state<Extract<ChannelView, { channel: "knowledge" }> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let loadingMore = $state(false);
	let request = 0;
	let session = 0;
	async function load(cursor: string | null, replace: boolean, showPaginationStatus = false) {
		if (loading) return;
		const version = ++request;
		loading = true;
		loadingMore = cursor !== null && showPaginationStatus;
		const response = await invoke<CommandResponse<ChannelView>>("read_channel", {
			query: {
				channel: "knowledge",
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
		if (response.data.channel !== "knowledge") {
			error = "The knowledge command returned the wrong channel.";
			return;
		}
		const page = response.data;
		if (!replace && content !== null)
			content = {
				...page,
				knowledge: [...content.knowledge, ...page.knowledge],
				newspaper: content.newspaper,
			};
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

		await load(null, true);
	}

	function reset() {
		session += 1;
		leave();
		content = null;
		error = null;
	}

	function initialize(response: CommandResponse<ChannelView>, version: number) {
		if (version !== request) return;
		if (response.status === "failed") {
			error = response.message;
			return;
		}
		if (response.data.channel !== "knowledge") {
			error = "The knowledge command returned the wrong channel.";
			return;
		}
		content = response.data;
		error = null;
	}
	async function readArticle(
		id: string,
		expectedHash: string | null,
	): Promise<CommandResponse<KnowledgeDocument>> {
		const version = session;
		const response = await invoke<CommandResponse<KnowledgeDocument>>("read_knowledge", {
			id,
			expectedHash,
		}).catch((): CommandResponse<KnowledgeDocument> => ({
			status: "failed",
			message: "Could not load the article. Please try again.",
		}));
		if (version !== session)
			return {
				status: "failed",
				message: "Knowledge configuration changed. Open the article again.",
			};
		return response;
	}
	async function createKnowledge(
		input: KnowledgeDraft,
	): Promise<CommandResponse<KnowledgeDocument>> {
		const version = session;
		const response = await invoke<CommandResponse<KnowledgeDocument>>("create_knowledge", {
			input,
		});
		if (version !== session) return response;
		if (response.status === "ready") leave();
		if (response.status === "ready" && content !== null) {
			content = {
				...content,
				knowledge: [
					response.data,
					...content.knowledge.filter((item) => item.id !== response.data.id),
				],
			};
			if (response.data.newspaperEdition !== null) void refresh();
		}
		return response;
	}

	async function updateKnowledge(
		id: string,
		input: KnowledgeUpdate,
	): Promise<CommandResponse<KnowledgeDocument>> {
		const version = session;
		const response = await invoke<CommandResponse<KnowledgeDocument>>("update_knowledge", {
			id,
			input,
		});
		if (version !== session) return response;
		if (response.status === "ready") leave();
		if (response.status === "ready" && content !== null) {
			content = {
				...content,
				knowledge: content.knowledge.map((document) =>
					document.id === id ? response.data : document,
				),
			};
			if (response.data.newspaperEdition !== null) void refresh();
		}
		return response;
	}

	onMount(() => {
		const timer = window.setInterval(() => {
			if (!context.active) return;

			if (!loading && context.mainElement !== null && context.mainElement.scrollTop < 200)
				void load(null, true);
		}, 60_000);
		const nextNewspaperRefresh = new SvelteDate();
		nextNewspaperRefresh.setHours(9, 0, 0, 0);
		if (nextNewspaperRefresh.getTime() <= Date.now())
			nextNewspaperRefresh.setDate(nextNewspaperRefresh.getDate() + 1);
		let newspaperTimer: number | null = null;
		const newspaperStartTimer = window.setTimeout(() => {
			void refresh();
			newspaperTimer = window.setInterval(() => void refresh(), 24 * 60 * 60 * 1_000);
		}, nextNewspaperRefresh.getTime() - Date.now());

		return () => {
			leave();
			window.clearInterval(timer);
			window.clearTimeout(newspaperStartTimer);
			if (newspaperTimer !== null) window.clearInterval(newspaperTimer);
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
		readArticle,
		createKnowledge,
		updateKnowledge,
	};
}
