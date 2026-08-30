<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { Archive, Bell, BookOpen, CloudOff, Heart, Home, Image, LayoutDashboard, Lock, Menu, Moon, Newspaper as NewspaperIcon, Settings, Sun, X } from "@lucide/svelte";
	import { onMount, tick } from "svelte";
	import MemosView from "./lib/components/MemosView.svelte";
	import MomentView from "./lib/components/MomentView.svelte";
	import KnowledgeView from "./lib/components/KnowledgeView.svelte";
	import InboxView from "./lib/components/InboxView.svelte";
	import NewspaperView from "./lib/components/NewspaperView.svelte";
	import DashboardView from "./lib/components/DashboardView.svelte";
	import SettingsView from "./lib/components/SettingsView.svelte";
	import ScrollToTop from "./lib/components/ScrollToTop.svelte";
	import type {
		ApiConfiguration,
		Channel,
		ChannelView,
		CommandResponse,
		ConfigurationStatus,
		DashboardEvent,
		DashboardState,
		InitialViews,
		NtfyConfig,
		NtfyNotification,
		KnowledgeDocument,
		KnowledgeDraft,
		KnowledgeUpdate,
		MemoTagCount,
		MemoView,
		MemoUpdate,
		PhotoUpdate,
		PhotoItem,
		QueryState,
		R2Configuration,
		TodoList,
		UgosConfiguration,
		UpdateInfo,
		UpdateProgress,
	} from "./lib/consumer";
	import { applyTheme, initTheme } from "./lib/theme";

	type View = "dashboard" | "inbox" | "newspaper" | "settings" | Channel;
	type MemoDisplay = "active" | "favorites" | "archived";
	const navigation: Array<{ id: View; label: string }> = [
		{ id: "dashboard", label: "Dashboard" },
		{ id: "newspaper", label: "Newspaper" },
		{ id: "memos", label: "Memos" },
		{ id: "moment", label: "Moment" },
		{ id: "knowledge", label: "Knowledge" },
		{ id: "settings", label: "Settings" },
	];
	const consumerChannels: Channel[] = ["memos", "moment", "knowledge"];
	const memoPageSize = 25;
	const defaultProfileAvatar = new URL("./assets/pleasure1234-avatar.png", import.meta.url).href;
	const profileNameKey = "vesper.profile.name";
	const profileAvatarKey = "vesper.profile.avatar";
	const sidebarWidthKey = "vesper.sidebar.width";
	const minimumSidebarWidth = 220;
	const maximumSidebarWidth = 360;

	let selected = $state<View>("dashboard");
	let previousView = $state<Exclude<View, "inbox">>("dashboard");
	let memoDisplay = $state<MemoDisplay>("active");
	let content = $state<ChannelView | null>(null);
	let error = $state<string | null>(null);
	let cache = $state<Record<Channel, ChannelView | null>>({
		memos: null,
		moment: null,
		knowledge: null,
	});
	let errors = $state<Record<Channel, string | null>>({
		memos: null,
		moment: null,
		knowledge: null,
	});
	let loading = $state(false);
	let loadingMore = $state(false);
	let mainElement = $state<HTMLElement | null>(null);
	let request = 0;
	let memoFilterRequest = 0;
	let memoSearch = "";
	let memoTags: string[] = [];
	let memoSortByUpdated = false;
	let memoTagIndex: MemoTagCount[] = [];
	let momentTagIndex: string[] = [];
	let dark = $state(initTheme());
	let sidebarOpen = $state(false);
	let dashboard = $state<DashboardState>({
		taskManager: { data: null, error: null, loading: false },
		codex: { data: null, error: null, loading: false },
		openCode: { data: null, error: null, loading: false },
		deepSeek: { data: null, error: null, loading: false },
		cherryIn: { data: null, error: null, loading: false },
		weather: { data: null, error: null, loading: false },
		stocks: { data: null, error: null, loading: false },
		github: { data: null, error: null, loading: false },
	});
	let dashboardRefreshing = $state(false);
	let todos = $state<QueryState<TodoList>>({ data: null, error: null, loading: false });
	const initialTodoDate = currentDate();
	let todayDate = $state(initialTodoDate);
	let todoDate = $state(initialTodoDate);
	let todoRequest = 0;
	let configuration = $state<ConfigurationStatus | null>(null);
	let configurationError = $state<string | null>(null);
	let notifications = $state<NtfyNotification[]>([]);
	let updateAvailable = $state<UpdateInfo | null>(null);
	let updateProgress = $state<UpdateProgress | null>(null);
	let updateError = $state<string | null>(null);
	let updateCheckError = $state<string | null>(null);
	let updateCheckNotice = $state<string | null>(null);
	let updateChecking = $state(false);
	let installingUpdate = $state(false);
	let updateDialog = $state<HTMLDivElement | null>(null);
	let updatePercent = $derived(
		updateProgress?.status === "downloading" && updateProgress.total !== null && updateProgress.total > 0
			? Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100))
			: null,
	);
	$effect(() => {
		if (updateAvailable === null || locked) return;
		void tick().then(() => updateDialog?.focus());
	});
	let locked = $state(false);
	let unlockPassword = $state("");
	let unlockError = $state<string | null>(null);
	let unlocking = $state(false);
	let unlockInput = $state<HTMLInputElement | null>(null);
	let profileName = $state("Pleasure1234");
	let profileAvatar = $state(defaultProfileAvatar);
	let profileEditing = $state(false);
	let profileNameDraft = $state("Pleasure1234");
	let profileAvatarDraft = $state(defaultProfileAvatar);
	let profileError = $state<string | null>(null);
	let profileAvatarInput = $state<HTMLInputElement | null>(null);
	let profileNameInput = $state<HTMLInputElement | null>(null);
	let profilePopover = $state<HTMLDivElement | null>(null);
	let sidebarWidth = $state(240);

	function currentDate() {
		return new Intl.DateTimeFormat("en-CA").format(new Date());
	}

	async function loadConfiguration() {
		configurationError = null;
		const response = await invoke<CommandResponse<ConfigurationStatus>>("read_configuration");
		if (response.status === "failed") {
			configurationError = response.message;
			return;
		}
		configuration = response.data;
	}

	async function checkForUpdate(manual = false) {
		if (updateChecking) {
			if (manual) {
				const message = "An update check is already running.";
				updateCheckNotice = message;
				window.setTimeout(() => {
					if (updateCheckNotice === message) updateCheckNotice = null;
				}, 5_000);
			}
			return;
		}
		updateChecking = true;
		updateCheckError = null;
		updateCheckNotice = null;
		const response = await invoke<CommandResponse<UpdateInfo | null>>("check_for_update");
		updateChecking = false;
		if (response.status === "failed") {
			updateCheckError = response.message;
			const message = response.message;
			window.setTimeout(() => {
				if (updateCheckError === message) updateCheckError = null;
			}, 5_000);
			return;
		}
		updateAvailable = response.data;
		if (manual && response.data === null) {
			const message = "Vesper is up to date.";
			updateCheckNotice = message;
			window.setTimeout(() => {
				if (updateCheckNotice === message) updateCheckNotice = null;
			}, 5_000);
		}
	}

	async function markNotificationRead(id: string) {
		const response = await invoke<CommandResponse<NtfyNotification[]>>("mark_notification_read", { id });
		if (response.status === "ready") notifications = response.data;
		return response;
	}

	async function installUpdate() {
		if (updateAvailable === null) return;
		installingUpdate = true;
		updateError = null;
		updateProgress = null;
		const response = await invoke<CommandResponse<string>>("install_update", {
			version: updateAvailable.version,
		});
		if (response.status === "failed") {
			installingUpdate = false;
			updateError = response.message;
		}
	}

	function keepUpdateDialogFocus(event: KeyboardEvent) {
		if (event.key !== "Tab" || updateDialog === null) return;
		const controls = updateDialog.querySelectorAll<HTMLElement>("button:not(:disabled)");
		if (controls.length === 0) return;
		const first = controls.item(0);
		const last = controls.item(controls.length - 1);
		if (document.activeElement === updateDialog) {
			event.preventDefault();
			(event.shiftKey ? last : first).focus();
		} else if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	}

	async function refreshDashboard() {
		if (dashboardRefreshing) return;
		dashboardRefreshing = true;
		for (const state of Object.values(dashboard)) {
			state.loading = true;
			state.error = null;
		}
		const [response] = await Promise.all([
			invoke<CommandResponse<null>>("refresh_dashboard"),
			loadTodos(todoDate),
		]);
		if (response.status === "failed") {
			for (const state of Object.values(dashboard)) {
				state.loading = false;
				state.error = response.message;
			}
		}
		dashboardRefreshing = false;
	}

	async function loadTodos(date = todoDate) {
		const version = ++todoRequest;
		todoDate = date;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("read_todos", { date });
		if (version !== todoRequest) return;
		todos.loading = false;
		if (response.status === "ready") todos.data = response.data;
		else todos.error = response.message;
	}

	async function addTodo(text: string): Promise<boolean> {
		if (todos.loading) return false;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("add_todo", { date, text });
		if (version !== todoRequest) return false;
		todos.loading = false;
		if (response.status === "failed") {
			todos.error = response.message;
			return false;
		}
		todos.data = response.data;
		return true;
	}

	async function toggleTodo(id: string, completed: boolean) {
		if (todos.loading) return;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("set_todo_completed", {
			date,
			id,
			completed,
		});
		if (version !== todoRequest) return;
		todos.loading = false;
		if (response.status === "ready") todos.data = response.data;
		else todos.error = response.message;
	}

	async function deleteTodo(id: string) {
		if (todos.loading) return;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("delete_todo", {
			date,
			id,
		});
		if (version !== todoRequest) return;
		todos.loading = false;
		if (response.status === "ready") todos.data = response.data;
		else todos.error = response.message;
	}

	async function select(view: View) {
		if (view === "inbox") {
			if (selected === "inbox") view = previousView;
			else previousView = selected;
		}
		request += 1;
		selected = view;
		void invoke<CommandResponse<null>>("set_dashboard_active", { active: view === "dashboard" });
		sidebarOpen = false;
		if (view === "dashboard" || view === "inbox" || view === "settings") {
			content = null;
			error = null;
			loading = false;
			loadingMore = false;
			if (view === "dashboard") void refreshDashboard();
			return;
		}
		const channel: Channel = view === "newspaper" ? "knowledge" : view;
		content = cache[channel];
		error = content === null ? errors[channel] : null;
		loading = false;
		loadingMore = false;
		if (content !== null && error === null) {
			if (view === "newspaper") {
				await load(channel, null, true, request);
				return;
			}
			await tick();
			if (
				mainElement !== null &&
				mainElement.scrollHeight - mainElement.scrollTop - mainElement.clientHeight < 600
			) loadMore();
			return;
		}
		await load(channel, null, true, request);
	}

	async function load(channel: Channel, cursor: string | null, replace: boolean, viewVersion: number) {
		if (loading) return;
		loading = true;
		loadingMore = cursor !== null;
		const response = await invoke<CommandResponse<ChannelView>>("read_channel", {
			query: {
				channel,
				cursor,
				search: channel === "memos" && memoSearch !== "" ? memoSearch : null,
				tags: channel === "memos" ? memoTags : [],
				sortByUpdated: channel === "memos" && memoSortByUpdated,
				archivedOnly: channel === "memos" && memoDisplay === "archived",
				favoritesOnly: channel === "memos" && memoDisplay === "favorites",
			},
		});
		if (viewVersion !== request) return;
		loading = false;
		loadingMore = false;
		if (response.status === "failed") {
			errors[channel] = response.message;
			if (content === null || content.channel !== channel) error = response.message;
			return;
		}
		const page = response.data.channel === "memos"
			? { ...response.data, tags: memoTagIndex }
			: response.data.channel === "moment"
				? { ...response.data, tags: momentTagIndex }
				: response.data;
		if (replace && content?.channel === "memos" && page.channel === "memos") {
			const tail = content.memos.slice(memoPageSize);
			const refreshedIds = new Set(page.memos.map((memo) => memo.id));
			content = {
				...page,
				memos: [...page.memos, ...tail.filter((memo) => !refreshedIds.has(memo.id))],
				nextCursor: tail.length > 0 ? content.nextCursor : page.nextCursor,
			};
		} else if (replace) {
			content = page;
		} else if (content?.channel === "memos" && page.channel === "memos") {
			content = { ...page, memos: [...content.memos, ...page.memos], tags: content.tags };
		} else if (content?.channel === "moment" && page.channel === "moment") {
			content = { ...page, photos: [...content.photos, ...page.photos], tags: content.tags };
		} else if (content?.channel === "knowledge" && page.channel === "knowledge") {
			content = { ...page, knowledge: [...content.knowledge, ...page.knowledge], newspaper: content.newspaper };
		}
		cache[channel] = content;
		errors[channel] = null;
		await tick();
		if (
			mainElement !== null &&
			mainElement.scrollHeight - mainElement.scrollTop - mainElement.clientHeight < 600
		) loadMore();
	}

	async function filterMemos(
		search: string,
		tags: string[],
		sortByUpdated: boolean,
		display: MemoDisplay,
	): Promise<string | null> {
		const version = ++memoFilterRequest;
		const requestVersion = ++request;
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
		if (version !== memoFilterRequest || requestVersion !== request || selected !== "memos") return null;
		loading = false;
		loadingMore = false;
		if (response.status === "failed") {
			return response.message;
		}
		if (response.data.channel !== "memos") return "The memo command returned the wrong channel.";
		const page = { ...response.data, tags: memoTagIndex };
		content = page;
		cache.memos = page;
		errors.memos = null;
		error = null;
		await tick();
		if (
			mainElement !== null &&
			mainElement.scrollHeight - mainElement.scrollTop - mainElement.clientHeight < 600
		) loadMore();
		return null;
	}

	async function revealMemo(id: string): Promise<boolean> {
		if (selected !== "memos" || content === null || content.channel !== "memos") return false;
		if (content.memos.some((memo) => memo.id === id)) return true;
		if (loading) return false;

		while (content.nextCursor !== null) {
			const cursor = content.nextCursor;
			await load("memos", cursor, false, request);
			if (content === null || content.channel !== "memos") return false;
			if (content.memos.some((memo) => memo.id === id)) return true;
			if (content.nextCursor === cursor) return false;
		}
		return false;
	}

	function loadMore() {
		if (selected === "dashboard" || selected === "inbox" || selected === "newspaper" || selected === "settings" || loading || content === null || content.nextCursor === null) return;
		void load(selected, content.nextCursor, false, request);
	}

	async function lockApp() {
		if (configuration?.appLock.status !== "ready") {
			await select("settings");
			return;
		}
		locked = true;
		sidebarOpen = false;
		unlockPassword = "";
		unlockError = null;
		await tick();
		unlockInput?.focus();
	}

	async function unlockApp(event: SubmitEvent) {
		event.preventDefault();
		if (unlocking || unlockPassword === "") return;
		unlocking = true;
		unlockError = null;
		const response = await invoke<CommandResponse<string>>("unlock_app", { password: unlockPassword });
		unlocking = false;
		if (response.status === "failed") {
			unlockError = response.message;
			unlockPassword = "";
			await tick();
			unlockInput?.focus();
			return;
		}
		unlockPassword = "";
		locked = false;
		void refreshConsumers();
	}

	async function refreshConsumers() {
		const version = ++request;
		loading = false;
		loadingMore = false;
		const responses = await Promise.all(
			consumerChannels.map(async (channel) => {
				return {
					channel,
					response: await invoke<CommandResponse<ChannelView>>("read_channel", {
						query: {
							channel,
							cursor: null,
							search: channel === "memos" && memoSearch !== "" ? memoSearch : null,
							tags: channel === "memos" ? memoTags : [],
							sortByUpdated: channel === "memos" && memoSortByUpdated,
							archivedOnly: channel === "memos" && memoDisplay === "archived",
							favoritesOnly: channel === "memos" && memoDisplay === "favorites",
						},
					}),
				};
			}),
		);
		if (version !== request) return;
		for (const { channel, response } of responses) {
			if (response.status === "failed") {
				errors[channel] = response.message;
				continue;
			}
			const page = response.data.channel === "memos"
				? { ...response.data, tags: memoTagIndex }
				: response.data.channel === "moment"
					? { ...response.data, tags: momentTagIndex }
					: response.data;
			cache[channel] = page;
			errors[channel] = null;
			const selectedChannel = selected === "newspaper" ? "knowledge" : selected;
			if (selectedChannel === channel) {
				content = page;
				error = null;
			}
		}
	}

	async function refreshKnowledge() {
		const response = await invoke<CommandResponse<ChannelView>>("read_channel", {
			query: {
				channel: "knowledge",
				cursor: null,
				search: null,
				tags: [],
				sortByUpdated: false,
				archivedOnly: false,
				favoritesOnly: false,
			},
		});
		if (response.status === "failed") {
			errors.knowledge = response.message;
			if (selected === "knowledge" || selected === "newspaper") error = response.message;
			return;
		}
		cache.knowledge = response.data;
		errors.knowledge = null;
		if (selected === "knowledge" || selected === "newspaper") {
			content = response.data;
			error = null;
		}
	}

	function loadProfile() {
		profileName = localStorage.getItem(profileNameKey) ?? "Pleasure1234";
		profileAvatar = localStorage.getItem(profileAvatarKey) ?? defaultProfileAvatar;
		const savedSidebarWidth = Number(localStorage.getItem(sidebarWidthKey));
		if (Number.isFinite(savedSidebarWidth) && savedSidebarWidth >= minimumSidebarWidth && savedSidebarWidth <= maximumSidebarWidth) {
			sidebarWidth = savedSidebarWidth;
		}
	}

	function beginSidebarResize(event: PointerEvent & { currentTarget: HTMLButtonElement }) {
		if (window.innerWidth < 768) return;
		const handle = event.currentTarget;
		const pointerId = event.pointerId;
		const startingX = event.clientX;
		const startingWidth = sidebarWidth;
		handle.setPointerCapture(pointerId);

		const resize = (moveEvent: PointerEvent) => {
			sidebarWidth = Math.min(
				maximumSidebarWidth,
				Math.max(minimumSidebarWidth, startingWidth + moveEvent.clientX - startingX),
			);
		};
		const finish = () => {
			handle.removeEventListener("pointermove", resize);
			handle.removeEventListener("pointerup", finish);
			handle.removeEventListener("pointercancel", finish);
			if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
			localStorage.setItem(sidebarWidthKey, String(sidebarWidth));
		};

		handle.addEventListener("pointermove", resize);
		handle.addEventListener("pointerup", finish);
		handle.addEventListener("pointercancel", finish);
	}

	function resizeSidebarByKey(event: KeyboardEvent) {
		if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
		event.preventDefault();
		const delta = event.key === "ArrowLeft" ? -8 : 8;
		sidebarWidth = Math.min(maximumSidebarWidth, Math.max(minimumSidebarWidth, sidebarWidth + delta));
		localStorage.setItem(sidebarWidthKey, String(sidebarWidth));
	}

	async function toggleProfileEditor() {
		profileEditing = !profileEditing;
		profileNameDraft = profileName;
		profileAvatarDraft = profileAvatar;
		profileError = null;
		if (profileEditing) {
			await tick();
			const input = profileNameInput;
			if (input !== null) {
				input.focus();
				input.setSelectionRange(input.value.length, input.value.length);
			}
		}
	}

	function closeProfileEditorOnBlur(event: FocusEvent) {
		const next = event.relatedTarget;
		if (next instanceof Node && profilePopover?.contains(next)) return;
		window.setTimeout(() => {
			if (profilePopover?.contains(document.activeElement)) return;
			profileEditing = false;
			profileError = null;
		}, 0);
	}

	async function changeProfileAvatar(input: HTMLInputElement) {
		const files = input.files;
		input.value = "";
		if (files === null) return;
		const file = files.item(0);
		if (file === null) return;
		if (!file.type.startsWith("image/")) {
			profileError = "Choose an image file.";
			return;
		}
		let image: ImageBitmap | null = null;
		try {
			image = await createImageBitmap(file);
			const canvas = document.createElement("canvas");
			canvas.width = 256;
			canvas.height = 256;
			const context = canvas.getContext("2d");
			if (context === null) {
				profileError = "The image processor is unavailable.";
				return;
			}
			const sourceSize = Math.min(image.width, image.height);
			context.drawImage(
				image,
				(image.width - sourceSize) / 2,
				(image.height - sourceSize) / 2,
				sourceSize,
				sourceSize,
				0,
				0,
				canvas.width,
				canvas.height,
			);
			profileAvatarDraft = canvas.toDataURL("image/png");
			profileError = null;
		} catch {
			profileError = "This image could not be opened.";
		} finally {
			image?.close();
		}
	}

	function saveProfile(event: SubmitEvent) {
		event.preventDefault();
		const name = profileNameDraft.trim();
		if (name === "") {
			profileError = "Enter a username.";
			return;
		}
		try {
			localStorage.setItem(profileNameKey, name);
			localStorage.setItem(profileAvatarKey, profileAvatarDraft);
		} catch {
			profileError = "The profile could not be saved on this device.";
			return;
		}
		profileName = name;
		profileAvatar = profileAvatarDraft;
		profileEditing = false;
	}

	function resetProfileDraft() {
		profileNameDraft = "Pleasure1234";
		profileAvatarDraft = defaultProfileAvatar;
		profileError = null;
	}

	function toggleTheme() {
		dark = !dark;
		applyTheme(dark);
	}

	async function createMemo(markdown: string, visibility: "public" | "private"): Promise<CommandResponse<MemoView>> {
		const response = await invoke<CommandResponse<MemoView>>("create_memo", {
			content: markdown,
			visibility,
		});
		if (response.status === "ready" && content !== null && content.channel === "memos") {
			content = { ...content, memos: [response.data, ...content.memos] };
			cache.memos = content;
		}
		return response;
	}

	async function importXMemo(url: string, visibility: "public" | "private"): Promise<CommandResponse<MemoView>> {
		const response = await invoke<CommandResponse<MemoView>>("import_x_memo", { url, visibility });
		if (response.status === "ready" && content !== null && content.channel === "memos") {
			content = { ...content, memos: [response.data, ...content.memos] };
			cache.memos = content;
		}
		return response;
	}

	async function updateMemo(id: string, input: MemoUpdate): Promise<CommandResponse<MemoView>> {
		const response = await invoke<CommandResponse<MemoView>>("update_memo", {
			id,
			input,
		});
		if (response.status === "ready" && content !== null && content.channel === "memos") {
			content = {
				...content,
				memos: content.memos.map((memo) => (memo.id === id ? response.data : memo)),
			};
			cache.memos = content;
		}
		return response;
	}

	async function deleteMemo(id: string): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("delete_memo", { id });
		if (response.status === "ready" && content !== null && content.channel === "memos") {
			content = { ...content, memos: content.memos.filter((memo) => memo.id !== id) };
			cache.memos = content;
		}
		return response;
	}

	function addUploadedPhoto(photo: PhotoItem) {
		if (content === null || content.channel !== "moment") return;
		content = {
			...content,
			photos: [photo, ...content.photos],
			tags: Array.from(new Set([...content.tags, ...photo.tags])).sort(),
			total: content.total + 1,
		};
		cache.moment = content;
	}

	async function updatePhoto(id: string, input: PhotoUpdate): Promise<CommandResponse<PhotoItem>> {
		const response = await invoke<CommandResponse<PhotoItem>>("update_photo", { id, input });
		if (response.status === "ready" && content !== null && content.channel === "moment") {
			content = {
				...content,
				photos: content.photos.map((photo) => (photo.id === id ? response.data : photo)),
				tags: Array.from(
					new Set(content.photos.flatMap((photo) => (photo.id === id ? response.data.tags : photo.tags))),
				).sort(),
			};
			cache.moment = content;
		}
		return response;
	}

	async function deletePhoto(id: string): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("delete_photo", { id });
		if (response.status === "ready" && content !== null && content.channel === "moment") {
			const photos = content.photos.filter((photo) => photo.id !== id);
			content = {
				...content,
				photos,
				tags: Array.from(new Set(photos.flatMap((photo) => photo.tags))).sort(),
				total: content.total - 1,
			};
			cache.moment = content;
		}
		return response;
	}

	async function createKnowledge(input: KnowledgeDraft): Promise<CommandResponse<KnowledgeDocument>> {
		const response = await invoke<CommandResponse<KnowledgeDocument>>("create_knowledge", { input });
		if (response.status === "ready" && content?.channel === "knowledge") {
			content = { ...content, knowledge: [response.data, ...content.knowledge] };
			cache.knowledge = content;
			if (response.data.newspaperEdition !== null) void refreshKnowledge();
		}
		return response;
	}

	async function updateKnowledge(
		id: string,
		input: KnowledgeUpdate,
	): Promise<CommandResponse<KnowledgeDocument>> {
		const response = await invoke<CommandResponse<KnowledgeDocument>>("update_knowledge", {
			id,
			input,
		});
		if (response.status === "ready" && content?.channel === "knowledge") {
			content = {
				...content,
				knowledge: content.knowledge.map((document) => (document.id === id ? response.data : document)),
			};
			cache.knowledge = content;
			if (response.data.newspaperEdition !== null) void refreshKnowledge();
		}
		return response;
	}

	async function saveUgosConfiguration(input: UgosConfiguration): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_ugos_configuration", {
			username: input.username,
			password: input.password,
		});
		if (response.status === "ready") {
			await loadConfiguration();
			await refreshDashboard();
		}
		return response;
	}

	async function saveR2Configuration(input: R2Configuration): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_r2_configuration", {
			accessKeyId: input.accessKeyId,
			secretAccessKey: input.secretAccessKey,
		});
		if (response.status === "ready") {
			await loadConfiguration();
			await initializeConsumers();
		}
		return response;
	}

	async function saveApiConfiguration(input: ApiConfiguration): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_api_configuration", {
			service: input.service,
			apiKey: input.apiKey,
		});
		if (response.status === "ready") {
			await loadConfiguration();
			await initializeConsumers();
		}
		return response;
	}

	async function saveNtfy(configuration: NtfyConfig): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_ntfy_configuration", {
			configuration,
		});
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function saveAppLock(password: string): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_app_lock", { password });
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function removeAppLock(): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("remove_app_lock");
		if (response.status === "ready") await loadConfiguration();
		return response;
	}

	async function initializeConsumers() {
		const initial = await invoke<InitialViews>("initialize_views");
		for (const id of consumerChannels) {
			const response = initial[id];
			if (response.status === "ready") {
				cache[id] = response.data.channel === "memos"
					? { ...response.data, tags: memoTagIndex }
					: response.data.channel === "moment"
						? { ...response.data, tags: momentTagIndex }
						: response.data;
				errors[id] = null;
			} else {
				errors[id] = response.message;
			}
		}
		if (selected === "dashboard" || selected === "inbox" || selected === "settings") {
			content = null;
			error = null;
		} else {
			const channel: Channel = selected === "newspaper" ? "knowledge" : selected;
			content = cache[channel];
			error = errors[channel];
		}
		await tick();
	}

	async function loadConsumerTags() {
		const [memos, moment] = await Promise.all([
			invoke<CommandResponse<MemoTagCount[]>>("read_memo_tags"),
			invoke<CommandResponse<string[]>>("read_moment_tags"),
		]);
		if (memos.status === "ready") {
			memoTagIndex = memos.data;
			if (cache.memos?.channel === "memos") cache.memos = { ...cache.memos, tags: memoTagIndex };
			if (content?.channel === "memos") content = { ...content, tags: memoTagIndex };
		}
		if (moment.status === "ready") {
			momentTagIndex = moment.data;
			if (cache.moment?.channel === "moment") cache.moment = { ...cache.moment, tags: momentTagIndex };
			if (content?.channel === "moment") content = { ...content, tags: momentTagIndex };
		}
	}

	onMount(() => {
		loadProfile();
		void loadConfiguration();
		const unlistenDashboard = listen<DashboardEvent>("dashboard-source-updated", (event) => {
			const update = event.payload;
			switch (update.source) {
				case "taskManager":
					dashboard.taskManager.loading = false;
					if (update.result.status === "ready") {
						dashboard.taskManager.data = update.result.data;
						dashboard.taskManager.error = null;
					}
					else dashboard.taskManager.error = update.result.message;
					break;
				case "codex":
					dashboard.codex.loading = false;
					if (update.result.status === "ready") {
						dashboard.codex.data = update.result.data;
						dashboard.codex.error = null;
					}
					else dashboard.codex.error = update.result.message;
					break;
				case "openCode":
					dashboard.openCode.loading = false;
					if (update.result.status === "ready") {
						dashboard.openCode.data = update.result.data;
						dashboard.openCode.error = null;
					}
					else dashboard.openCode.error = update.result.message;
					break;
				case "deepSeek":
					dashboard.deepSeek.loading = false;
					if (update.result.status === "ready") {
						dashboard.deepSeek.data = update.result.data;
						dashboard.deepSeek.error = null;
					}
					else dashboard.deepSeek.error = update.result.message;
					break;
				case "cherryIn":
					dashboard.cherryIn.loading = false;
					if (update.result.status === "ready") {
						dashboard.cherryIn.data = update.result.data;
						dashboard.cherryIn.error = null;
					}
					else dashboard.cherryIn.error = update.result.message;
					break;
				case "weather":
					dashboard.weather.loading = false;
					if (update.result.status === "ready") {
						dashboard.weather.data = update.result.data;
						dashboard.weather.error = null;
					}
					else dashboard.weather.error = update.result.message;
					break;
				case "stocks":
					dashboard.stocks.loading = false;
					if (update.result.status === "ready") {
						dashboard.stocks.data = update.result.data;
						dashboard.stocks.error = null;
					}
					else dashboard.stocks.error = update.result.message;
					break;
				case "github":
					dashboard.github.loading = false;
					if (update.result.status === "ready") {
						dashboard.github.data = update.result.data;
						dashboard.github.error = null;
					}
					else dashboard.github.error = update.result.message;
			}
		}).then((unlisten) => {
			void invoke<CommandResponse<null>>("set_dashboard_active", { active: true });
			void refreshDashboard();
			return unlisten;
		});
		void invoke<CommandResponse<NtfyNotification[]>>("read_notifications").then((response) => {
			if (response.status === "ready") notifications = response.data;
		});
		const unlistenTodo = listen<TodoList>("todo-list-changed", (event) => {
			const followsToday = todoDate === todayDate;
			todayDate = event.payload.date;
			if (followsToday) {
				todoRequest += 1;
				todoDate = event.payload.date;
				todos.data = event.payload;
				todos.error = null;
				todos.loading = false;
			}
		});
		const unlistenUpdater = listen<UpdateProgress>("updater-progress", (event) => {
			updateProgress = event.payload;
		});
		const unlistenUpdateRequest = listen("check-for-updates-requested", () => {
			void checkForUpdate(true);
		});
		const unlistenNotifications = listen<NtfyNotification[]>("notifications-updated", (event) => {
			notifications = event.payload;
		});
		void checkForUpdate();
		void initializeConsumers().then(loadConsumerTags);
		const todoTimer = window.setInterval(() => {
			if (!todos.loading) void loadTodos();
		}, 60_000);
		const contentTimer = window.setInterval(() => {
			if (
				(selected === "memos" || selected === "moment" || selected === "knowledge" || selected === "newspaper") &&
				!loading &&
				mainElement !== null &&
				mainElement.scrollTop < 200
			) {
				const channel: Channel = selected === "newspaper" ? "knowledge" : selected;
				void load(channel, null, true, request);
			}
		}, 60_000);
		const nextNewspaperRefresh = new Date();
		nextNewspaperRefresh.setHours(9, 0, 0, 0);
		if (nextNewspaperRefresh.getTime() <= Date.now()) nextNewspaperRefresh.setDate(nextNewspaperRefresh.getDate() + 1);
		let newspaperTimer: number | null = null;
		const newspaperStartTimer = window.setTimeout(() => {
			void refreshKnowledge();
			newspaperTimer = window.setInterval(() => void refreshKnowledge(), 24 * 60 * 60 * 1_000);
		}, nextNewspaperRefresh.getTime() - Date.now());
		return () => {
			void invoke<CommandResponse<null>>("set_dashboard_active", { active: false });
			void unlistenDashboard.then((unlisten) => unlisten());
			window.clearInterval(todoTimer);
			void unlistenTodo.then((unlisten) => unlisten());
			void unlistenUpdater.then((unlisten) => unlisten());
			void unlistenUpdateRequest.then((unlisten) => unlisten());
			void unlistenNotifications.then((unlisten) => unlisten());
			window.clearInterval(contentTimer);
			window.clearTimeout(newspaperStartTimer);
			if (newspaperTimer !== null) window.clearInterval(newspaperTimer);
		};
	});
</script>

<svelte:head>
	<title>Vesper</title>
	<meta name="description" content="Local previews for Memos and Moment." />
</svelte:head>

<div class="shell" class:locked inert={locked || updateAvailable !== null} style:--sidebar-width={`${sidebarWidth}px`}>
	<button
		type="button"
		class:open={sidebarOpen}
		class="sidebar-overlay"
		onclick={() => (sidebarOpen = false)}
		aria-label="Close sidebar"
	></button>

	<aside class:open={sidebarOpen}>
		<div class="sidebar-header">
			<button type="button" class="close-sidebar" onclick={() => (sidebarOpen = false)} aria-label="Close sidebar">
				<X size={15} />
			</button>
		</div>

		<nav aria-label="Consumer views">
			{#each navigation as item}
				<button
					type="button"
					class:active={selected === item.id}
					aria-current={selected === item.id ? "page" : "false"}
					onclick={() => void select(item.id)}
				>
					{#if item.id === "dashboard"}<LayoutDashboard size={15} />{:else if item.id === "memos"}<Home size={15} />{:else if item.id === "moment"}<Image size={15} />{:else if item.id === "newspaper"}<NewspaperIcon size={15} />{:else if item.id === "knowledge"}<BookOpen size={15} />{:else}<Settings size={15} />{/if}
					{item.label}
				</button>
			{/each}
		</nav>

		<div class="sidebar-rule"></div>

		<div class="storage">
			<p>Connections</p>
			<div><span class:offline={configuration === null || configuration.api.memos.status === "missing"}></span>my-memos API</div>
			<div><span class:offline={configuration === null || configuration.api.moment.status === "missing"}></span>my-moment API</div>
			<div><span class:offline={configuration === null || configuration.api.knowledge.status === "missing"}></span>my-knowledge API</div>
			<div><span class:offline={configuration === null || configuration.r2.status === "missing"}></span>Cloudflare R2</div>
			{#if configuration === null}<small>Checking credential store</small>{:else}<small>Managed in Settings</small>{/if}
		</div>

		<div class="sidebar-footer">
			<div class="profile-popover-anchor" bind:this={profilePopover} onfocusout={closeProfileEditorOnBlur}>
				{#if profileEditing}
					<div class="profile-editor" role="dialog" aria-label="Edit local profile">
					<form onsubmit={saveProfile}>
						<div class="profile-editor-heading">
							<img src={profileAvatarDraft} alt="Profile preview" />
							<div><strong>Local profile</strong><span>Display only</span></div>
						</div>
						<label for="profile-name">Username</label>
						<input id="profile-name" bind:this={profileNameInput} maxlength="24" autocomplete="off" bind:value={profileNameDraft} />
						<input class="avatar-input" bind:this={profileAvatarInput} type="file" accept="image/*" onchange={(event) => void changeProfileAvatar(event.currentTarget)} />
						<div class="profile-editor-actions">
							<button type="button" onclick={() => profileAvatarInput?.click()}>Change photo</button>
							<button type="button" onclick={resetProfileDraft}>Reset</button>
							<button type="submit">Save</button>
						</div>
						{#if profileError !== null}<p role="alert">{profileError}</p>{/if}
					</form>
					</div>
				{/if}
				<button class="user-profile" type="button" onclick={toggleProfileEditor} aria-haspopup="dialog" aria-expanded={profileEditing} aria-label={`Edit local profile for ${profileName}`} title="Edit local profile">
					<img src={profileAvatar} alt="" />
					<span>{profileName}</span>
				</button>
			</div>
			<div class="footer-controls">
				<div class="footer-navigation">
					<button
						class:active={selected === "inbox"}
						type="button"
						onclick={() => void select("inbox")}
						aria-label={selected === "inbox"
							? "Return to previous view"
							: notifications.length > 0
								? `Open inbox, ${notifications.length} unread notifications`
								: "Open inbox"}
						title={selected === "inbox" ? "Back" : "Inbox"}
					>
						<Bell size={15} />
						{#if notifications.length > 0}<span class="notification-dot" aria-hidden="true"></span>{/if}
					</button>
				</div>
				<div class="footer-actions">
					<button type="button" onclick={lockApp} aria-label={configuration?.appLock.status === "ready" ? "Lock Vesper" : "Configure App Lock"} title={configuration?.appLock.status === "ready" ? "Lock Vesper" : "Configure App Lock in Settings"}>
						<Lock size={15} />
					</button>
					<button type="button" onclick={toggleTheme} aria-label={dark ? "Switch to light mode" : "Switch to dark mode"} title={dark ? "Light mode" : "Dark mode"}>
						{#if dark}<Sun size={15} />{:else}<Moon size={15} />{/if}
					</button>
				</div>
			</div>
		</div>
		<button
			type="button"
			class="sidebar-resizer"
			aria-label={`Resize sidebar, currently ${sidebarWidth} pixels wide`}
			onpointerdown={beginSidebarResize}
			onkeydown={resizeSidebarByKey}
		></button>
	</aside>

	<main
		bind:this={mainElement}
		onscroll={() => {
			if (
				mainElement !== null &&
				mainElement.scrollHeight - mainElement.scrollTop - mainElement.clientHeight < 600
			) loadMore();
		}}
	>
		<header class="topbar">
			<button class="menu-button" type="button" onclick={() => (sidebarOpen = true)} aria-label="Open sidebar">
				<Menu size={18} />
			</button>
			<strong>vesper</strong>
		</header>
		<div class="canvas">
			{#if selected === "dashboard"}
				<DashboardView
					snapshot={dashboard.taskManager.data}
					error={dashboard.taskManager.error}
					refreshing={dashboardRefreshing}
					usage={dashboard.codex.data}
					usageError={dashboard.codex.error}
					openCodeUsage={dashboard.openCode.data}
					openCodeUsageError={dashboard.openCode.error}
					deepSeekBalance={dashboard.deepSeek.data}
					deepSeekBalanceError={dashboard.deepSeek.error}
					cherryInUsage={dashboard.cherryIn.data}
					cherryInUsageError={dashboard.cherryIn.error}
					weather={dashboard.weather.data}
					weatherError={dashboard.weather.error}
					stocks={dashboard.stocks.data}
					stocksError={dashboard.stocks.error}
					github={dashboard.github.data}
					githubError={dashboard.github.error}
					todos={todos.data}
					todosError={todos.error}
					todosLoading={todos.loading}
					todayDate={todayDate}
					todoDate={todoDate}
					onselecttododate={loadTodos}
					onaddtodo={addTodo}
					ontoggletodo={toggleTodo}
					ondeletetodo={deleteTodo}
					onrefresh={refreshDashboard}
				/>
			{:else if selected === "settings"}
				<SettingsView {configuration} error={configurationError} onsaveugos={saveUgosConfiguration} onsaver2={saveR2Configuration} onsaveapi={saveApiConfiguration} onsaventfy={saveNtfy} onsaveapplock={saveAppLock} onremoveapplock={removeAppLock} />
			{:else if selected === "inbox"}
				<InboxView {notifications} onread={markNotificationRead} />
			{:else if error}
				<section class="consumer-error">
					<header>
						<p>{selected === "moment" ? "Cloudflare R2" : "Consumer API"}</p>
						{#if selected === "memos"}<h1>Memos</h1>{:else if selected === "moment"}<h1>Moment</h1>{:else if selected === "newspaper"}<h1>Newspaper</h1>{:else}<h1>Knowledge</h1>{/if}
					</header>
					<div class="error" role="alert">
						<CloudOff size={18} />
						<div><strong>Content unavailable</strong><span>{error}</span></div>
						<button type="button" onclick={() => void select("settings")}>Open Settings</button>
					</div>
				</section>
			{:else if content !== null}
				{#if content.channel === "memos"}
					<MemosView memos={content.memos} tags={content.tags} display={memoDisplay} onfilter={filterMemos} onopenmemo={revealMemo} oncreate={createMemo} onimportx={importXMemo} onupdate={updateMemo} ondelete={deleteMemo} />
				{:else if content.channel === "moment"}
					<MomentView photos={content.photos} tags={content.tags} total={content.total} onuploaded={addUploadedPhoto} onupdate={updatePhoto} ondelete={deletePhoto} />
				{:else if selected === "newspaper"}
					<NewspaperView documents={content.knowledge} issues={content.newspaper} {loading} />
				{:else}
					<KnowledgeView documents={content.knowledge} {loading} oncreate={createKnowledge} onupdate={updateKnowledge} />
				{/if}
			{:else}
				<div class="loading" aria-live="polite" aria-label="Loading view">
					<div class="loading-card">
						<div class="loading-meta"><span></span><span></span></div>
						<div class="loading-copy"><span></span><span></span><span></span></div>
						<div class="loading-footer"><span></span><span></span></div>
					</div>
					<div class="loading-card secondary" aria-hidden="true">
						<div class="loading-meta"><span></span><span></span></div>
						<div class="loading-copy"><span></span><span></span></div>
					</div>
				</div>
			{/if}
			<div class="sentinel" aria-hidden="true"></div>
			{#if loadingMore && content}
				<p class="loading-more">Loading more…</p>
			{/if}
		</div>
		<div class="global-scroll-action">
			{#if selected === "memos"}
				<button
					class="memo-filter-action"
					class:active={memoDisplay === "archived"}
					type="button"
					onclick={() => (memoDisplay = memoDisplay === "archived" ? "active" : "archived")}
					aria-pressed={memoDisplay === "archived"}
					aria-label={memoDisplay === "archived" ? "Show active memos" : "Show archived memos"}
					title={memoDisplay === "archived" ? "Active memos" : "Archived memos"}
				>
					<Archive size={15} />
				</button>
				<button
					class="memo-filter-action"
					class:active={memoDisplay === "favorites"}
					type="button"
					onclick={() => (memoDisplay = memoDisplay === "favorites" ? "active" : "favorites")}
					aria-pressed={memoDisplay === "favorites"}
					aria-label={memoDisplay === "favorites" ? "Show active memos" : "Show favorite memos"}
					title={memoDisplay === "favorites" ? "Active memos" : "Favorite memos"}
				>
					<Heart size={15} fill={memoDisplay === "favorites" ? "currentColor" : "none"} />
				</button>
			{/if}
			<ScrollToTop />
		</div>
	</main>
</div>

{#if locked}
	<div class="lock-screen" role="dialog" aria-modal="true" aria-labelledby="lock-title">
		<div class="lock-card">
			<h1 id="lock-title">Locked</h1>
			<form onsubmit={unlockApp}>
				<label for="unlock-password">Password</label>
				<input bind:this={unlockInput} id="unlock-password" type="password" bind:value={unlockPassword} autocomplete="current-password" placeholder="Enter password" />
				{#if unlockError}<p class="unlock-error" role="alert">{unlockError}</p>{/if}
				<button type="submit" disabled={unlocking || unlockPassword === ""}>{unlocking ? "Unlocking…" : "Unlock"}</button>
			</form>
		</div>
	</div>
{/if}

{#if updateAvailable !== null && !locked}
	<div class="update-overlay" role="presentation">
		<div bind:this={updateDialog} class="update-dialog" role="dialog" aria-modal="true" aria-labelledby="update-title" tabindex="-1" onkeydown={keepUpdateDialogFocus}>
			<p>Application update</p>
			<h1 id="update-title">Vesper {updateAvailable.version} is available</h1>
			<span>Installed version: {updateAvailable.currentVersion}</span>
			{#if updateAvailable.notes}<div class="update-notes">{updateAvailable.notes}</div>{/if}
			{#if installingUpdate}
				<div class="update-progress" class:indeterminate={updatePercent === null} role="progressbar" aria-label="Application update download" aria-valuemin="0" aria-valuemax="100" aria-valuenow={updatePercent}>
					<span style:width={updatePercent === null ? "100%" : `${updatePercent}%`}></span>
				</div>
				<small>{updateProgress?.status === "downloaded" ? "Installing and restarting…" : updatePercent === null ? "Downloading update…" : `Downloading update… ${updatePercent}%`}</small>
			{/if}
			{#if updateError}<div class="update-error" role="alert">{updateError}</div>{/if}
			<div class="update-actions">
				<button type="button" disabled={installingUpdate} onclick={() => (updateAvailable = null)}>Later</button>
				<button class="primary" type="button" disabled={installingUpdate} onclick={() => void installUpdate()}>{installingUpdate ? "Updating…" : "Download and restart"}</button>
			</div>
		</div>
	</div>
{/if}

{#if updateCheckError !== null && updateAvailable === null && !locked}
	<div class="update-check-feedback error" role="alert">
		<span>{updateCheckError}</span>
		<button type="button" aria-label="Dismiss update error" onclick={() => (updateCheckError = null)}>×</button>
	</div>
{/if}

{#if updateCheckNotice !== null && updateAvailable === null && updateCheckError === null && !locked}
	<div class="update-check-feedback" role="status">
		<span>{updateCheckNotice}</span>
		<button type="button" aria-label="Dismiss update status" onclick={() => (updateCheckNotice = null)}>×</button>
	</div>
{/if}

<style>
	:global(html),
	:global(body),
	:global(#app) {
		width: 100%;
		height: 100%;
	}

	:global(body) {
		margin: 0;
		min-width: 320px;
		overflow: hidden;
		background: var(--color-background);
		color: var(--color-foreground);
		font-family: var(--font-sans);
	}

	:global(button) {
		font-family: inherit;
	}

	.shell {
		display: grid;
		grid-template-columns: var(--sidebar-width, 15rem) minmax(0, 1fr);
		height: 100vh;
		overflow: hidden;
	}

	.shell.locked { filter: blur(1rem); }

	.update-overlay {
		position: fixed;
		inset: 0;
		z-index: 120;
		display: grid;
		place-items: center;
		padding: 1rem;
		background: var(--color-overlay);
	}

	.update-dialog {
		display: grid;
		width: min(28rem, 100%);
		box-sizing: border-box;
		gap: 0.75rem;
		padding: 1.25rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-lg);
	}

	.update-dialog p,
	.update-dialog h1,
	.update-dialog span,
	.update-dialog small { margin: 0; }
	.update-dialog p { color: var(--color-accent); font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	.update-dialog h1 { font-family: var(--font-serif); font-size: 1.35rem; font-weight: 500; }
	.update-dialog > span,
	.update-dialog small { color: var(--color-muted-foreground); font-size: 0.72rem; }
	.update-notes { max-height: 10rem; overflow: auto; white-space: pre-wrap; font-size: 0.78rem; line-height: 1.6; }
	.update-progress { height: 0.35rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.update-progress span { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); }
	.update-progress.indeterminate span { animation: update-pulse 1.2s ease-in-out infinite alternate; }
	.update-error { color: var(--color-destructive); font-size: 0.75rem; }
	.update-actions { display: flex; justify-content: flex-end; gap: 0.5rem; padding-top: 0.25rem; }
	.update-actions button { height: 2rem; padding: 0 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); cursor: pointer; font-size: 0.72rem; }
	.update-actions button.primary { border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.update-actions button:disabled { cursor: not-allowed; opacity: 0.6; }
	.update-check-feedback { position: fixed; right: 1rem; bottom: 1rem; z-index: 110; display: flex; max-width: min(32rem, calc(100vw - 2rem)); align-items: center; gap: 0.625rem; padding: 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-lg); }
	.update-check-feedback span { color: var(--color-foreground); font-size: 0.75rem; line-height: 1.4; }
	.update-check-feedback.error { border-color: var(--color-error); }
	.update-check-feedback.error span { color: var(--color-error); }
	.update-check-feedback button { padding: 0.25rem 0.5rem; border: 0; border-radius: var(--radius-sm); background: var(--color-muted); color: var(--color-foreground); cursor: pointer; font-size: 0.7rem; }
	@keyframes update-pulse { from { opacity: 0.35; } to { opacity: 1; } }

	.sidebar-overlay {
		display: none;
	}

	aside {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100vh;
		box-sizing: border-box;
		border-right: 1px solid var(--color-border);
		background: var(--color-background);
	}

	.sidebar-header { display: none; }

	.sidebar-resizer {
		position: absolute;
		top: 0;
		right: -0.2rem;
		bottom: 0;
		z-index: 4;
		width: 0.4rem;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: col-resize;
		touch-action: none;
	}

	.sidebar-resizer::after {
		position: absolute;
		top: 0;
		bottom: 0;
		left: calc(50% - 0.5px);
		width: 1px;
		background: var(--color-accent);
		content: "";
		opacity: 0;
	}

	.sidebar-resizer:hover::after,
	.sidebar-resizer:focus-visible::after { opacity: 1; }

	.close-sidebar {
		display: none;
		width: 1.75rem;
		height: 1.75rem;
		margin-left: auto;
		place-items: center;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
	}

	nav {
		display: grid;
		gap: 0.125rem;
		padding: 1.25rem 0.75rem 0;
	}

	nav button {
		display: flex;
		align-items: center;
		height: 2.25rem;
		gap: 0.625rem;
		padding: 0 0.75rem;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 400;
		text-align: left;
	}

	nav button:hover,
	nav button.active {
		border-color: transparent;
		background: color-mix(in srgb, var(--color-accent) 10%, transparent);
		color: var(--color-accent);
	}

	.sidebar-rule {
		height: 1px;
		margin: 1rem;
		background: var(--color-border);
	}

	.storage {
		display: grid;
		gap: 0.45rem;
		padding: 0 1.25rem;
		color: var(--color-muted-foreground);
		font-size: 0.75rem;
	}

	.storage p {
		margin: 0 0 0.15rem;
		font-size: 0.68rem;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.storage div {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-foreground);
	}

	.storage div span {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: var(--radius-full);
		background: var(--color-success);
	}

	.storage div span.offline {
		background: var(--color-muted-foreground);
	}

	.storage small {
		padding-left: 0.95rem;
		font-size: 0.68rem;
	}

	.user-profile {
		display: flex;
		flex: 1 1 auto;
		align-items: center;
		gap: 0.375rem;
		min-width: 0;
		height: 2rem;
		padding: 0 0.125rem;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		cursor: pointer;
		text-align: left;
	}

	.profile-popover-anchor {
		position: relative;
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
	}

	.user-profile:hover { background: var(--color-muted); }

	.user-profile img {
		width: 1.75rem;
		height: 1.75rem;
		flex: 0 0 auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		object-fit: cover;
	}

	.user-profile span {
		overflow: hidden;
		color: var(--color-foreground);
		font-size: 0.65rem;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sidebar-footer {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: auto;
		padding: 0.625rem 0.5rem;
		border-top: 1px solid var(--color-border);
	}

	.profile-editor {
		position: absolute;
		bottom: calc(100% + 0.75rem);
		left: 0;
		z-index: 30;
		display: grid;
		width: min(17rem, calc(100vw - 2rem));
		box-sizing: border-box;
		gap: 0.5rem;
		padding: 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-lg);
	}
	.profile-editor form { display: contents; }

	.profile-editor-heading { display: flex; align-items: center; gap: 0.625rem; }
	.profile-editor-heading img { width: 2.5rem; height: 2.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-full); object-fit: cover; }
	.profile-editor-heading div { display: grid; gap: 0.1rem; }
	.profile-editor-heading strong { font-size: 0.75rem; font-weight: 600; }
	.profile-editor-heading span,
	.profile-editor label { color: var(--color-muted-foreground); font-size: 0.65rem; }
	.profile-editor input:not(.avatar-input) { min-width: 0; height: 1.9rem; box-sizing: border-box; padding: 0 0.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font-size: 0.72rem; }
	.profile-editor input:not(.avatar-input):focus { border-color: var(--color-accent); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 14%, transparent); }
	.avatar-input { display: none; }
	.profile-editor-actions { display: flex; gap: 0.3rem; }
	.profile-editor-actions button { height: 1.7rem; padding: 0 0.45rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: transparent; color: var(--color-foreground); cursor: pointer; font-size: 0.62rem; }
	.profile-editor-actions button:last-child { margin-left: auto; border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.profile-editor p { margin: 0; color: var(--color-error); font-size: 0.62rem; }

	.footer-controls {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
	}

	.footer-navigation,
	.footer-actions {
		display: flex;
		align-items: center;
		gap: 0.0625rem;
	}

	.footer-actions {
		padding-left: 0.0625rem;
	}

	.lock-screen {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: grid;
		place-items: center;
		padding: 1.5rem;
		background: color-mix(in srgb, var(--color-background) 96%, var(--color-muted));
	}

	.lock-card {
		display: grid;
		width: min(100%, 18rem);
		justify-items: center;
		gap: 0.9rem;
		box-sizing: border-box;
		padding: 1rem;
		text-align: center;
	}

	.lock-card h1 { margin: 0 0 0.35rem; font-size: 1rem; font-weight: 600; }
	.lock-card form { display: grid; width: 100%; gap: 0.55rem; text-align: left; }
	.lock-card label { color: var(--color-muted-foreground); font-size: 0.65rem; }
	.lock-card input { min-width: 0; height: 2rem; box-sizing: border-box; padding: 0 0.625rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font-size: 0.75rem; }
	.lock-card input:focus { border-color: var(--color-accent); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 14%, transparent); }
	.lock-card button { height: 1.75rem; margin-top: 0.15rem; padding: 0 0.625rem; border: 1px solid var(--color-accent); border-radius: var(--radius-md); background: var(--color-accent); color: var(--color-accent-foreground); cursor: pointer; font-size: 0.68rem; font-weight: 400; }
	.lock-card button:disabled { cursor: wait; opacity: 0.55; }
	.unlock-error { margin: 0.2rem 0 0; color: var(--color-error); font-size: 0.68rem; }

	.footer-controls button,
	.topbar button {
		position: relative;
		display: grid;
		width: 1.65rem;
		height: 1.65rem;
		place-items: center;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
	}

	.notification-dot {
		position: absolute;
		top: 0.25rem;
		right: 0.25rem;
		width: 0.375rem;
		height: 0.375rem;
		border: 1px solid var(--color-background);
		border-radius: var(--radius-full);
		background: var(--color-error);
	}

	.footer-controls button:hover,
	.topbar button:hover,
	.close-sidebar:hover {
		background: var(--color-muted);
		color: var(--color-foreground);
	}
	.footer-controls button.active { background: color-mix(in srgb, var(--color-accent) 10%, transparent); color: var(--color-accent); }

	main {
		min-width: 0;
		height: 100vh;
		overflow-y: auto;
	}

	.topbar {
		position: sticky;
		top: 0;
		z-index: 10;
		display: none;
		height: 3rem;
		align-items: center;
		justify-content: flex-end;
		padding: 0 0.75rem;
		border-bottom: 1px solid var(--color-border);
		background: color-mix(in srgb, var(--color-background) 92%, transparent);
		backdrop-filter: blur(12px);
	}

	.topbar .menu-button,
	.topbar strong { display: none; }

	.canvas {
		width: min(100% - 2rem, 66rem);
		margin: 0 auto;
		padding: 2rem 1rem 5rem;
		box-sizing: border-box;
	}

	.global-scroll-action {
		position: fixed;
		right: 1.25rem;
		bottom: 1.25rem;
		z-index: 30;
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.memo-filter-action {
		display: grid;
		width: 2.75rem;
		height: 2.75rem;
		padding: 0;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
		border-radius: var(--radius-full);
		background: color-mix(in srgb, var(--color-background) 82%, transparent);
		box-shadow: var(--shadow-sm);
		color: var(--color-muted-foreground);
		cursor: pointer;
		backdrop-filter: blur(14px);
		transition:
			border-color var(--duration-fast),
			background var(--duration-fast),
			color var(--duration-fast),
			translate var(--duration-fast);
	}

	.memo-filter-action:hover {
		border-color: var(--color-border-strong);
		background: color-mix(in srgb, var(--color-background) 94%, var(--color-muted));
		color: var(--color-foreground);
		translate: 0 -2px;
	}

	.memo-filter-action.active {
		border-color: color-mix(in srgb, var(--color-accent) 55%, var(--color-border));
		background: color-mix(in srgb, var(--color-accent) 12%, var(--color-background));
		color: var(--color-accent);
	}

	.memo-filter-action:focus-visible {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.consumer-error {
		width: min(100%, 64rem);
		margin: 0 auto;
	}

	.consumer-error header {
		margin-bottom: 1.5rem;
	}

	.consumer-error header p,
	.consumer-error header h1 {
		margin: 0;
	}

	.consumer-error header p {
		margin-bottom: 0.35rem;
		color: var(--color-accent);
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.consumer-error header h1 {
		font-family: var(--font-serif);
		font-size: 2rem;
		font-weight: 500;
	}

	.error {
		display: flex;
		min-height: 12rem;
		align-items: center;
		justify-content: center;
		gap: 0.65rem;
		box-sizing: border-box;
		padding: 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-muted-foreground);
	}

	.error div {
		display: grid;
		gap: 0.2rem;
	}

	.error strong,
	.error span {
		font-size: 0.8rem;
	}

	.error button {
		height: 2rem;
		margin-left: 0.35rem;
		padding: 0 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-background);
		color: var(--color-foreground);
		cursor: pointer;
		font-size: 0.72rem;
	}

	.loading {
		display: grid;
		width: min(100%, 42rem);
		min-height: calc(100vh - 10rem);
		align-content: center;
		gap: 0.75rem;
		margin: 0 auto;
	}

	.loading-card {
		display: grid;
		gap: 1rem;
		padding: 1rem 1.25rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
	}

	.loading-card.secondary {
		opacity: 0.65;
	}

	.loading-meta,
	.loading-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.loading-copy {
		display: grid;
		gap: 0.6rem;
	}

	.loading-footer {
		padding-top: 0.75rem;
		border-top: 1px solid var(--color-border);
	}

	.loading span {
		display: block;
		height: 0.55rem;
		border-radius: var(--radius-full);
		background: linear-gradient(90deg, var(--color-muted) 25%, var(--color-border) 50%, var(--color-muted) 75%);
		background-size: 220% 100%;
		animation: shimmer var(--duration-skeleton) ease-in-out infinite;
	}

	.loading-meta span:first-child { width: 5rem; }
	.loading-meta span:last-child { width: 3.25rem; height: 1.25rem; }
	.loading-copy span:nth-child(1) { width: 94%; }
	.loading-copy span:nth-child(2) { width: 78%; }
	.loading-copy span:nth-child(3) { width: 52%; }
	.loading-footer span:first-child { width: 4rem; height: 1.5rem; }
	.loading-footer span:last-child { width: 4.5rem; height: 1.5rem; }

	@keyframes shimmer {
		from { background-position: 100% 0; }
		to { background-position: -120% 0; }
	}

	.sentinel {
		height: 1px;
	}

	.loading-more {
		margin: 1.5rem 0 0;
		color: var(--color-muted-foreground);
		font-size: 0.75rem;
		text-align: center;
	}

	@media (max-width: 767px) {
		.shell {
			grid-template-columns: 1fr;
		}

		aside {
			position: fixed;
			inset: 0 auto 0 0;
			z-index: 30;
			width: 15rem;
			translate: -100% 0;
			transition: translate var(--duration-fast);
		}

		aside.open {
			translate: 0 0;
		}

		.sidebar-overlay.open {
			position: fixed;
			inset: 0;
			z-index: 20;
			display: block;
			border: 0;
			background: var(--color-overlay);
		}

		.close-sidebar {
			display: grid;
		}

		.sidebar-resizer { display: none; }

		.sidebar-header {
			display: flex;
			justify-content: flex-end;
			padding: 0.75rem 0.75rem 0.25rem;
		}

		nav { padding-top: 0.25rem; }

		.topbar {
			display: flex;
			gap: 0.5rem;
			justify-content: flex-start;
		}

		.topbar .menu-button { display: grid; }

		.topbar strong {
			display: block;
			font-family: var(--font-serif);
			font-size: 0.95rem;
		}

		.canvas {
			width: auto;
			padding: 1rem 1rem 3rem;
		}
	}
</style>
