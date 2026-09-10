import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type {
	CommandResponse,
	DashboardState,
	DashboardEvent,
	QueryState,
	TodoList,
} from "../../consumer";

export function createDashboardSession(
	isActive: () => boolean,
	surface: "dashboard" | "island" = "dashboard",
) {
	let dashboard = $state<DashboardState>({
		taskManager: { data: null, error: null, loading: false },
		deviceTelemetry: { data: null, error: null, loading: false },
		codex: { data: null, error: null, loading: false },
		openCode: { data: null, error: null, loading: false },
		claude: { data: null, error: null, loading: false },
		grok: { data: null, error: null, loading: false },
		copilot: { data: null, error: null, loading: false },
		deepSeek: { data: null, error: null, loading: false },
		cherryIn: { data: null, error: null, loading: false },
		weather: { data: null, error: null, loading: false },
		stocks: { data: null, error: null, loading: false },
		exchange: { data: null, error: null, loading: false },
		serviceStatus: { data: null, error: null, loading: false },
		github: { data: null, error: null, loading: false },
		quotation: { data: null, error: null, loading: false },
	});
	let dashboardRefreshing = $state(false);
	let dashboardRequest = 0;
	let todos = $state<QueryState<TodoList>>({ data: null, error: null, loading: false });
	const initialTodoDate = new Intl.DateTimeFormat("en-CA").format(new Date());
	let todayDate = $state(initialTodoDate);
	let todoDate = $state(initialTodoDate);
	let todoRequest = 0;
	let todoInvalidated = false;
	let todoWriting = false;
	let todoWriteError: string | null = null;
	async function refreshDashboard(refreshGames = false) {
		if (!isActive() || dashboardRefreshing) return;
		const version = ++dashboardRequest;
		dashboardRefreshing = true;
		for (const state of Object.values(dashboard)) {
			state.loading = true;
			state.error = null;
		}
		const [response] = await Promise.all([
			invoke<CommandResponse<null>>("refresh_dashboard", { refreshGames }),
			loadTodos(todoDate),
		]);
		if (version !== dashboardRequest) return;
		if (response.status === "failed") {
			for (const state of Object.values(dashboard)) {
				state.loading = false;
				state.error = response.message;
			}
		}
		dashboardRefreshing = false;
	}

	async function loadTodos(date = todoDate) {
		if (date !== todoDate) todoWriteError = null;
		if (todoWriting) {
			todoDate = date;
			todoInvalidated = true;
			todos.loading = true;
			return;
		}
		const version = ++todoRequest;
		todoDate = date;
		todos.loading = true;
		todos.error = todoWriteError;
		const response = await invoke<CommandResponse<TodoList>>("read_todos", { date });
		if (version !== todoRequest) return;
		todos.loading = false;
		if (todoInvalidated) {
			todoInvalidated = false;
			void loadTodos();
		}
		if (response.status === "ready") {
			todos.data = response.data;
			todos.error = todoWriteError === null ? response.data.syncError : todoWriteError;
		} else
			todos.error =
				todoWriteError === null ? response.message : `${todoWriteError} · ${response.message}`;
	}

	function finishTodoMutation(
		version: number,
		date: string,
		response: CommandResponse<TodoList>,
	): boolean {
		todoWriting = false;
		if (version !== todoRequest) {
			if (todoInvalidated) {
				todoInvalidated = false;
				void loadTodos();
			}
			return false;
		}
		todos.loading = false;
		const sameDate = todoDate === date;
		if (sameDate) {
			if (response.status === "ready") {
				todos.data = response.data;
				todos.error = response.data.syncError;
			} else {
				todoWriteError = response.message;
				todos.error = response.message;
			}
		}
		if (todoInvalidated) {
			todoInvalidated = false;
			void loadTodos();
		}
		return sameDate && response.status === "ready";
	}
	async function addTodo(text: string, description: string): Promise<boolean> {
		if (todos.loading || todoWriting) return false;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("add_todo", {
			date,
			text,
			description,
		});
		return finishTodoMutation(version, date, response);
	}

	async function editTodo(id: string, text: string, description: string): Promise<boolean> {
		if (todos.loading || todoWriting) return false;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("update_todo", {
			date,
			id,
			text,
			description,
		});
		return finishTodoMutation(version, date, response);
	}

	async function toggleTodo(id: string, completed: boolean) {
		if (todos.loading || todoWriting) return;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("set_todo_completed", {
			date,
			id,
			completed,
		});
		finishTodoMutation(version, date, response);
	}

	async function deleteTodo(id: string) {
		if (todos.loading || todoWriting) return;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = todoDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("delete_todo", {
			date,
			id,
		});
		finishTodoMutation(version, date, response);
	}

	async function activate(active: boolean) {
		if (!active) {
			dashboardRequest += 1;
			dashboardRefreshing = false;
		}
		await invoke<CommandResponse<null>>("set_dashboard_active", { active });
		if (active) void refreshDashboard();
	}
	onMount(() => {
		let disposed = false;
		const unlistenDashboard = listen<DashboardEvent>("dashboard-source-updated", (event) => {
			const update = event.payload;
			switch (update.source) {
				case "taskManager":
					dashboard.taskManager.loading = false;
					if (update.result.status === "ready") {
						dashboard.taskManager.data = update.result.data;
						dashboard.taskManager.error = null;
					} else dashboard.taskManager.error = update.result.message;
					break;
				case "deviceTelemetry":
					dashboard.deviceTelemetry.loading = false;
					if (update.result.status === "ready") {
						dashboard.deviceTelemetry.data = update.result.data;
						dashboard.deviceTelemetry.error = null;
					} else dashboard.deviceTelemetry.error = update.result.message;
					break;
				case "codex":
					dashboard.codex.loading = false;
					if (update.result.status === "ready") {
						dashboard.codex.data = update.result.data;
						dashboard.codex.error = null;
					} else dashboard.codex.error = update.result.message;
					break;
				case "openCode":
					dashboard.openCode.loading = false;
					if (update.result.status === "ready") {
						dashboard.openCode.data = update.result.data;
						dashboard.openCode.error = null;
					} else dashboard.openCode.error = update.result.message;
					break;
				case "claude":
					dashboard.claude.loading = false;
					if (update.result.status === "ready") {
						dashboard.claude.data = update.result.data;
						dashboard.claude.error = null;
					} else dashboard.claude.error = update.result.message;
					break;
				case "grok":
					dashboard.grok.loading = false;
					if (update.result.status === "ready") {
						dashboard.grok.data = update.result.data;
						dashboard.grok.error = null;
					} else dashboard.grok.error = update.result.message;
					break;
				case "copilot":
					dashboard.copilot.loading = false;
					if (update.result.status === "ready") {
						dashboard.copilot.data = update.result.data;
						dashboard.copilot.error = null;
					} else dashboard.copilot.error = update.result.message;
					break;
				case "deepSeek":
					dashboard.deepSeek.loading = false;
					if (update.result.status === "ready") {
						dashboard.deepSeek.data = update.result.data;
						dashboard.deepSeek.error = null;
					} else dashboard.deepSeek.error = update.result.message;
					break;
				case "cherryIn":
					dashboard.cherryIn.loading = false;
					if (update.result.status === "ready") {
						dashboard.cherryIn.data = update.result.data;
						dashboard.cherryIn.error = null;
					} else dashboard.cherryIn.error = update.result.message;
					break;
				case "weather":
					dashboard.weather.loading = false;
					if (update.result.status === "ready") {
						dashboard.weather.data = update.result.data;
						dashboard.weather.error = null;
					} else dashboard.weather.error = update.result.message;
					break;
				case "stocks":
					dashboard.stocks.loading = false;
					if (update.result.status === "ready") {
						dashboard.stocks.data = update.result.data;
						dashboard.stocks.error = null;
					} else dashboard.stocks.error = update.result.message;
					break;
				case "exchange":
					dashboard.exchange.loading = false;
					if (update.result.status === "ready") {
						dashboard.exchange.data = update.result.data;
						dashboard.exchange.error = null;
					} else dashboard.exchange.error = update.result.message;
					break;
				case "serviceStatus":
					dashboard.serviceStatus.loading = false;
					if (update.result.status === "ready") {
						dashboard.serviceStatus.data = update.result.data;
						dashboard.serviceStatus.error = null;
					} else dashboard.serviceStatus.error = update.result.message;
					break;
				case "github":
					dashboard.github.loading = false;
					if (update.result.status === "ready") {
						dashboard.github.data = update.result.data;
						dashboard.github.error = null;
					} else dashboard.github.error = update.result.message;
					break;
				case "quotation":
					dashboard.quotation.loading = false;
					if (update.result.status === "ready") {
						dashboard.quotation.data = update.result.data;
						dashboard.quotation.error = null;
					} else dashboard.quotation.error = update.result.message;
			}
		}).then(async (unlisten) => {
			if (!disposed && isActive()) {
				await invoke<CommandResponse<null>>("set_dashboard_active", { active: true });
				if (!disposed) void refreshDashboard();
			}
			return unlisten;
		});
		const unlistenTodo = listen<TodoList>("todo-list-changed", (event) => {
			const followsToday = todoDate === todayDate;
			todayDate = event.payload.date;
			if (followsToday) {
				if (todoDate !== event.payload.date) todoWriteError = null;
				todoRequest += 1;
				todoDate = event.payload.date;
				todos.data = event.payload;
				todos.error = todoWriteError;
				todos.loading = false;
			}
		});
		const unlistenUpdates = listen<string>("todo-updated", (event) => {
			if (event.payload !== todoDate) return;
			if (todos.loading) todoInvalidated = true;
			else void loadTodos();
		});
		const todoTimer = window.setInterval(() => {
			if (isActive() && !todos.loading) void loadTodos();
		}, 60_000);
		return () => {
			disposed = true;
			todoRequest += 1;
			dashboardRequest += 1;
			todoInvalidated = false;
			if (surface === "dashboard")
				void invoke<CommandResponse<null>>("set_dashboard_active", { active: false });
			void unlistenDashboard.then((unlisten) => unlisten());
			void unlistenTodo.then((unlisten) => unlisten());
			void unlistenUpdates.then((unlisten) => unlisten());
			window.clearInterval(todoTimer);
		};
	});
	return {
		get dashboard() {
			return dashboard;
		},
		get dashboardRefreshing() {
			return dashboardRefreshing;
		},
		get todos() {
			return todos;
		},
		get todayDate() {
			return todayDate;
		},
		get todoDate() {
			return todoDate;
		},
		activate,
		refreshDashboard,
		loadTodos,
		addTodo,
		editTodo,
		toggleTodo,
		deleteTodo,
	};
}
