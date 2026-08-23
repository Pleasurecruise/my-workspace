<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { BookOpen, CloudOff, Home, Image, LayoutDashboard, Menu, Moon, Settings, Sun, X } from "@lucide/svelte";
	import { onMount, tick } from "svelte";
	import MemosView from "./lib/components/MemosView.svelte";
	import MomentView from "./lib/components/MomentView.svelte";
	import KnowledgeView from "./lib/components/KnowledgeView.svelte";
	import DashboardView from "./lib/components/DashboardView.svelte";
	import SettingsView from "./lib/components/SettingsView.svelte";
	import type {
		ApiConfigurationInput,
		Channel,
		ChannelView,
		CommandResponse,
		CompiledKnowledge,
		ConfigurationStatus,
		DashboardQueryResults,
		DashboardState,
		InitialViews,
		MemoView,
		MemoUpdateInput,
		QueryState,
		R2ConfigurationInput,
		TodoList,
		UgosConfigurationInput,
	} from "./lib/consumer";
	import { applyTheme, initTheme } from "./lib/theme";

	type View = "dashboard" | "settings" | Channel;
	const navigation: Array<{ id: View; label: string }> = [
		{ id: "dashboard", label: "Dashboard" },
		{ id: "memos", label: "Memos" },
		{ id: "moment", label: "Moment" },
		{ id: "knowledge", label: "Knowledge" },
		{ id: "settings", label: "Settings" },
	];
	const consumerChannels: Channel[] = ["memos", "moment", "knowledge"];
	const memoPageSize = 25;

	let selected = $state<View>("dashboard");
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
	let request = 0;
	let sentinel = $state<HTMLDivElement | null>(null);
	let dark = $state(initTheme());
	let sidebarOpen = $state(false);
	let dashboard = $state<DashboardState>({
		taskManager: { data: null, error: null, loading: false },
		codex: { data: null, error: null, loading: false },
		openCode: { data: null, error: null, loading: false },
		deepSeek: { data: null, error: null, loading: false },
		cherryIn: { data: null, error: null, loading: false },
		weather: { data: null, error: null, loading: false },
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

	async function loadQuery<K extends keyof DashboardQueryResults>(
		command: K,
		state: QueryState<NoInfer<DashboardQueryResults[K]>>,
	) {
		if (state.loading) return;
		state.loading = true;
		state.error = null;
		const response = await invoke<CommandResponse<DashboardQueryResults[K]>>(command);
		state.loading = false;
		if (response.status === "ready") state.data = response.data;
		else state.error = response.message;
	}

	async function refreshDashboard() {
		if (dashboardRefreshing) return;
		dashboardRefreshing = true;
		await Promise.all([
			loadQuery("read_task_manager", dashboard.taskManager),
			loadQuery("read_codex_usage", dashboard.codex),
			loadQuery("read_opencode_usage", dashboard.openCode),
			loadQuery("read_deepseek_balance", dashboard.deepSeek),
			loadQuery("read_cherryin_balance", dashboard.cherryIn),
			loadQuery("read_weather", dashboard.weather),
			loadQuery("read_github", dashboard.github),
		]);
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
		request += 1;
		selected = view;
		sidebarOpen = false;
		if (view === "dashboard" || view === "settings") {
			content = null;
			error = null;
			loading = false;
			if (view === "dashboard") void loadQuery("read_github", dashboard.github);
			return;
		}
		const channel = view;
		content = cache[channel];
		error = content === null ? errors[channel] : null;
		loading = false;
		if (channel !== "memos" && (content !== null || error !== null)) return;
		await load(channel, null, true, request);
	}

	async function load(channel: Channel, cursor: string | null, replace: boolean, version: number) {
		if (loading) return;
		loading = true;
		const response = await invoke<CommandResponse<ChannelView>>("read_consumer_channel", {
			channel,
			cursor,
		});
		loading = false;
		if (version !== request) return;
		if (response.status === "failed") {
			errors[channel] = response.message;
			if (content === null || content.channel !== channel) error = response.message;
			return;
		}
		const page = response.data;
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
			content = { ...page, memos: [...content.memos, ...page.memos] };
		} else if (content?.channel === "moment" && page.channel === "moment") {
			content = { ...page, photos: [...content.photos, ...page.photos] };
		} else if (content?.channel === "knowledge" && page.channel === "knowledge") {
			content = { ...page, knowledge: [...content.knowledge, ...page.knowledge] };
		}
		cache[channel] = content;
		errors[channel] = null;
		await tick();
		if (sentinel && sentinel.getBoundingClientRect().top < window.innerHeight + 600) loadMore();
	}

	function loadMore() {
		if (selected === "dashboard" || selected === "settings" || loading || content === null || content.nextCursor === null) return;
		void load(selected, content.nextCursor, false, request);
	}

	function toggleTheme() {
		dark = !dark;
		applyTheme(dark);
	}

	async function createMemo(markdown: string, visibility: "public" | "private"): Promise<CommandResponse<MemoView>> {
		const response = await invoke<CommandResponse<MemoView>>("create_consumer_memo", {
			content: markdown,
			visibility,
		});
		if (response.status === "ready" && content !== null && content.channel === "memos") {
			content = { ...content, memos: [response.data, ...content.memos] };
			cache.memos = content;
		}
		return response;
	}

	async function updateMemo(id: string, input: MemoUpdateInput): Promise<CommandResponse<MemoView>> {
		const response = await invoke<CommandResponse<MemoView>>("update_consumer_memo", {
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
		const response = await invoke<CommandResponse<string>>("delete_consumer_memo", { id });
		if (response.status === "ready" && content !== null && content.channel === "memos") {
			content = { ...content, memos: content.memos.filter((memo) => memo.id !== id) };
			cache.memos = content;
		}
		return response;
	}

	function compileKnowledge(source: string): Promise<CommandResponse<CompiledKnowledge>> {
		return invoke<CommandResponse<CompiledKnowledge>>("compile_knowledge", { source });
	}

	async function saveUgosConfiguration(input: UgosConfigurationInput): Promise<CommandResponse<string>> {
		const response = await invoke<CommandResponse<string>>("save_ugos_configuration", {
			username: input.username,
			password: input.password,
		});
		if (response.status === "ready") {
			await loadConfiguration();
			await loadQuery("read_task_manager", dashboard.taskManager);
		}
		return response;
	}

	async function saveR2Configuration(input: R2ConfigurationInput): Promise<CommandResponse<string>> {
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

	async function saveApiConfiguration(input: ApiConfigurationInput): Promise<CommandResponse<string>> {
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

	async function initializeConsumers() {
		const initial = await invoke<InitialViews>("initialize_consumer_views");
		for (const id of consumerChannels) {
			const response = initial[id];
			if (response.status === "ready") {
				cache[id] = response.data;
				errors[id] = null;
			} else {
				errors[id] = response.message;
			}
		}
		if (selected === "dashboard" || selected === "settings") {
			content = null;
			error = null;
		} else {
			content = cache[selected];
			error = errors[selected];
		}
		await tick();
	}

	onMount(() => {
		void loadConfiguration();
		void loadQuery("read_task_manager", dashboard.taskManager);
		void loadQuery("read_codex_usage", dashboard.codex);
		void loadQuery("read_opencode_usage", dashboard.openCode);
		void loadQuery("read_deepseek_balance", dashboard.deepSeek);
		void loadQuery("read_cherryin_balance", dashboard.cherryIn);
		void loadQuery("read_weather", dashboard.weather);
		void loadQuery("read_github", dashboard.github);
		void loadTodos();
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
		void initializeConsumers();
		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (entry.isIntersecting) loadMore();
				}
			},
			{ rootMargin: "600px" },
		);
		if (sentinel) observer.observe(sentinel);
		const taskManagerTimer = window.setInterval(() => {
			if (selected === "dashboard") void loadQuery("read_task_manager", dashboard.taskManager);
		}, 2_000);
		const subscriptionUsageTimer = window.setInterval(() => {
			if (selected !== "dashboard") return;
			void loadQuery("read_codex_usage", dashboard.codex);
			void loadQuery("read_opencode_usage", dashboard.openCode);
			void loadQuery("read_deepseek_balance", dashboard.deepSeek);
			void loadQuery("read_cherryin_balance", dashboard.cherryIn);
		}, 60_000);
		const todoTimer = window.setInterval(() => {
			if (!todos.loading) void loadTodos();
		}, 60_000);
		const weatherTimer = window.setInterval(() => {
			if (selected === "dashboard") void loadQuery("read_weather", dashboard.weather);
		}, 900_000);
		const memosTimer = window.setInterval(() => {
			if (selected === "memos" && !loading) void load("memos", null, true, request);
		}, 60_000);
		return () => {
			observer.disconnect();
			window.clearInterval(taskManagerTimer);
			window.clearInterval(subscriptionUsageTimer);
			window.clearInterval(todoTimer);
			void unlistenTodo.then((unlisten) => unlisten());
			window.clearInterval(weatherTimer);
			window.clearInterval(memosTimer);
		};
	});
</script>

<svelte:head>
	<title>Vesper</title>
	<meta name="description" content="Local previews for Memos and Moment." />
</svelte:head>

<div class="shell">
	<button
		type="button"
		class:open={sidebarOpen}
		class="sidebar-overlay"
		onclick={() => (sidebarOpen = false)}
		aria-label="Close sidebar"
	></button>

	<aside class:open={sidebarOpen}>
		<div class="brand">
			<span class="brand-mark" aria-hidden="true">◆</span>
			<strong>vesper</strong>
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
					{#if item.id === "dashboard"}<LayoutDashboard size={15} />{:else if item.id === "memos"}<Home size={15} />{:else if item.id === "moment"}<Image size={15} />{:else if item.id === "knowledge"}<BookOpen size={15} />{:else}<Settings size={15} />{/if}
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
			<button type="button" onclick={toggleTheme} aria-label={dark ? "Switch to light mode" : "Switch to dark mode"}>
				{#if dark}<Sun size={15} />{:else}<Moon size={15} />{/if}
			</button>
		</div>
	</aside>

	<main>
		<header class="mobile-header">
			<button type="button" onclick={() => (sidebarOpen = true)} aria-label="Open sidebar">
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
				<SettingsView {configuration} error={configurationError} onsaveugos={saveUgosConfiguration} onsaver2={saveR2Configuration} onsaveapi={saveApiConfiguration} />
			{:else if error}
				<section class="consumer-error">
					<header>
						<p>{selected === "moment" ? "Cloudflare R2" : "Consumer API"}</p>
						{#if selected === "memos"}<h1>Memos</h1>{:else if selected === "moment"}<h1>Moment</h1>{:else}<h1>Knowledge</h1>{/if}
					</header>
					<div class="error" role="alert">
						<CloudOff size={18} />
						<div><strong>Content unavailable</strong><span>{error}</span></div>
						<button type="button" onclick={() => void select("settings")}>Open Settings</button>
					</div>
				</section>
			{:else if content !== null}
				{#if content.channel === "memos"}
					<MemosView memos={content.memos} oncreate={createMemo} onupdate={updateMemo} ondelete={deleteMemo} />
				{:else if content.channel === "moment"}
					<MomentView photos={content.photos} total={content.total} />
				{:else}
					<KnowledgeView documents={content.knowledge} {loading} oncompile={compileKnowledge} />
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
			<div bind:this={sentinel} class="sentinel" aria-hidden="true"></div>
			{#if loading && content}
				<p class="loading-more">Loading more…</p>
			{/if}
		</div>
	</main>
</div>

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
		grid-template-columns: 15rem minmax(0, 1fr);
		height: 100vh;
		overflow: hidden;
	}

	.sidebar-overlay {
		display: none;
	}

	aside {
		display: flex;
		flex-direction: column;
		height: 100vh;
		box-sizing: border-box;
		border-right: 1px solid var(--color-border);
		background: var(--color-background);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 1.25rem 1.25rem 1rem;
		color: var(--color-accent);
	}

	.brand-mark {
		display: grid;
		width: 1rem;
		height: 1rem;
		place-items: center;
		color: var(--color-accent);
		font-size: 0.65rem;
	}

	.brand strong {
		font-family: var(--font-serif);
		font-size: 1.15rem;
		letter-spacing: -0.02em;
	}

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
		padding: 0 0.75rem;
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

	.sidebar-footer {
		display: flex;
		justify-content: flex-end;
		margin-top: auto;
		padding: 1rem 0.75rem;
		border-top: 1px solid var(--color-border);
	}

	.sidebar-footer button,
	.mobile-header button {
		display: grid;
		width: 2rem;
		height: 2rem;
		place-items: center;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
	}

	.sidebar-footer button:hover,
	.mobile-header button:hover,
	.close-sidebar:hover {
		background: var(--color-muted);
		color: var(--color-foreground);
	}

	main {
		min-width: 0;
		height: 100vh;
		overflow-y: auto;
	}

	.mobile-header {
		display: none;
	}

	.canvas {
		width: min(100% - 2rem, 66rem);
		margin: 0 auto;
		padding: 2rem 1rem 5rem;
		box-sizing: border-box;
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

		.mobile-header {
			position: sticky;
			top: 0;
			z-index: 10;
			display: flex;
			height: 3rem;
			align-items: center;
			gap: 0.5rem;
			padding: 0 0.75rem;
			border-bottom: 1px solid var(--color-border);
			background: color-mix(in srgb, var(--color-background) 92%, transparent);
			backdrop-filter: blur(12px);
		}

		.mobile-header strong {
			font-family: var(--font-serif);
			font-size: 0.95rem;
		}

		.canvas {
			width: auto;
			padding: 1rem 1rem 3rem;
		}
	}
</style>
