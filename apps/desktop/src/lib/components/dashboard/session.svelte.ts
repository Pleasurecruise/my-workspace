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

function applySource<T>(state: QueryState<T>, response: CommandResponse<T | null>) {
	state.loading = false;
	if (response.status === "ready") {
		state.data = response.data;
		state.error = null;
	} else state.error = response.message;
}

export function createDashboardSession(
	isActive: () => boolean,
	surface: "dashboard" | "island" = "dashboard",
) {
	const dashboard = $state<DashboardState>({
		taskManager: { data: null, error: null, loading: false },
		deviceTelemetry: { data: null, error: null, loading: false },
		codex: { data: null, error: null, loading: false },
		openCode: { data: null, error: null, loading: false },
		claude: { data: null, error: null, loading: false },
		grok: { data: null, error: null, loading: false },
		copilot: { data: null, error: null, loading: false },
		deepSeek: { data: null, error: null, loading: false },
		cherryIn: { data: null, error: null, loading: false },
		tokenFlux: { data: null, error: null, loading: false },
		dimAgent: { data: null, error: null, loading: false },
		weather: { data: null, error: null, loading: false },
		stocks: { data: null, error: null, loading: false },
		exchange: { data: null, error: null, loading: false },
		serviceStatus: { data: null, error: null, loading: false },
		github: { data: null, error: null, loading: false },
		quotation: { data: null, error: null, loading: false },
	});
	let dashboardRefreshing = $state(false);
	let dashboardRequest = 0;
	const todos = $state<QueryState<TodoList>>({ data: null, error: null, loading: false });
	const initialDate = new Intl.DateTimeFormat("en-CA").format(Date.now());
	let todayDate = $state(initialDate);
	let selectedDate = $state(initialDate);
	let todoRequest = 0;
	let todoInvalidated = false;
	let todoRefresh = false;
	let todoWriting = false;
	let todoWriteError: string | null = null;
	let dateRequest = 0;
	function acceptDate(date: string) {
		const followsToday = selectedDate === todayDate;
		todayDate = date;
		if (followsToday && selectedDate !== date) {
			selectedDate = date;
			todoWriteError = null;
		}
	}
	async function refreshPlanner(refresh = false) {
		const version = ++dateRequest;
		const response = await invoke<CommandResponse<string>>("read_planner_date");
		if (version !== dateRequest) return;
		if (response.status === "ready") acceptDate(response.data);
		await loadTodos(refresh);
		if (version === dateRequest && response.status === "failed") todos.error = response.message;
	}
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
			refreshPlanner(refreshGames),
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

	function selectDate(date: string) {
		if (date === selectedDate) return;
		selectedDate = date;
		todoWriteError = null;
		todos.error = null;
	}

	async function loadTodos(refresh = false) {
		const date = selectedDate;
		todoRefresh ||= refresh;
		if (todoWriting) {
			todoInvalidated = true;
			todos.loading = true;
			return;
		}
		const version = ++todoRequest;
		todos.loading = true;
		todos.error = todoWriteError;
		const forceRefresh = todoRefresh;
		todoRefresh = false;
		const response = await invoke<CommandResponse<TodoList>>(
			"read_todos",
			forceRefresh ? { date, refresh: true } : { date },
		);
		if (version !== todoRequest || date !== selectedDate) return;
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

	function finishTodo(version: number, date: string, response: CommandResponse<TodoList>): boolean {
		todoWriting = false;
		if (version !== todoRequest) {
			if (todoInvalidated) {
				todoInvalidated = false;
				void loadTodos();
			}
			return false;
		}
		todos.loading = false;
		const sameDate = selectedDate === date;
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
		const date = selectedDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("add_todo", {
			date,
			text,
			description,
		});
		return finishTodo(version, date, response);
	}

	async function editTodo(id: string, text: string, description: string): Promise<boolean> {
		if (todos.loading || todoWriting) return false;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = selectedDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("update_todo", {
			date,
			id,
			text,
			description,
		});
		return finishTodo(version, date, response);
	}

	async function toggleTodo(id: string, completed: boolean) {
		if (todos.loading || todoWriting) return;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = selectedDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("set_todo_completed", {
			date,
			id,
			completed,
		});
		finishTodo(version, date, response);
	}

	async function setTodoRollover(id: string, rollover: boolean) {
		if (todos.loading || todoWriting) return;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = selectedDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("set_todo_rollover", {
			date,
			id,
			rollover,
		}).catch((): CommandResponse<TodoList> => ({
			status: "failed",
			message: "Could not save the carry-forward preference. Try again.",
		}));
		finishTodo(version, date, response);
	}

	async function reorderTodos(ids: string[]) {
		if (todos.loading || todoWriting) return false;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = selectedDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("reorder_todos", {
			date,
			ids,
		}).catch((): CommandResponse<TodoList> => ({
			status: "failed",
			message: "Could not save the Todo order. Try again.",
		}));
		return finishTodo(version, date, response);
	}

	async function deleteTodo(id: string) {
		if (todos.loading || todoWriting) return;
		todoWriting = true;
		todoWriteError = null;
		const version = ++todoRequest;
		const date = selectedDate;
		todos.loading = true;
		todos.error = null;
		const response = await invoke<CommandResponse<TodoList>>("delete_todo", {
			date,
			id,
		});
		finishTodo(version, date, response);
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
					applySource(dashboard.taskManager, update.result);
					break;
				case "deviceTelemetry":
					applySource(dashboard.deviceTelemetry, update.result);
					break;
				case "codex":
					applySource(dashboard.codex, update.result);
					break;
				case "openCode":
					applySource(dashboard.openCode, update.result);
					break;
				case "claude":
					applySource(dashboard.claude, update.result);
					break;
				case "grok":
					applySource(dashboard.grok, update.result);
					break;
				case "copilot":
					applySource(dashboard.copilot, update.result);
					break;
				case "deepSeek":
					applySource(dashboard.deepSeek, update.result);
					break;
				case "cherryIn":
					applySource(dashboard.cherryIn, update.result);
					break;
				case "tokenFlux":
					applySource(dashboard.tokenFlux, update.result);
					break;
				case "dimAgent":
					applySource(dashboard.dimAgent, update.result);
					break;
				case "weather":
					applySource(dashboard.weather, update.result);
					break;
				case "stocks":
					applySource(dashboard.stocks, update.result);
					break;
				case "exchange":
					applySource(dashboard.exchange, update.result);
					break;
				case "serviceStatus":
					applySource(dashboard.serviceStatus, update.result);
					break;
				case "github":
					applySource(dashboard.github, update.result);
					break;
				case "quotation":
					applySource(dashboard.quotation, update.result);
					break;
			}
		}).then(async (unlisten) => {
			if (!disposed && isActive()) {
				await invoke<CommandResponse<null>>("set_dashboard_active", { active: true });
				if (!disposed) void refreshDashboard();
			}
			return unlisten;
		});
		const unlistenTodo = listen<string>("planner-date-changed", (event) => {
			++dateRequest;
			acceptDate(event.payload);
			void loadTodos();
		});
		const unlistenUpdates = listen<string>("todo-updated", (event) => {
			if (event.payload !== selectedDate) return;
			if (todos.loading) todoInvalidated = true;
			else void loadTodos();
		});
		const todoTimer = window.setInterval(() => {
			if (isActive() && !todos.loading) void refreshPlanner();
		}, 60_000);
		const focus = () => {
			if (isActive()) void refreshPlanner();
		};
		window.addEventListener("focus", focus);
		return () => {
			disposed = true;
			++dateRequest;
			window.removeEventListener("focus", focus);
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
		get selectedDate() {
			return selectedDate;
		},
		activate,
		refreshDashboard,
		refreshPlanner,
		selectDate,
		loadTodos,
		addTodo,
		editTodo,
		toggleTodo,
		setTodoRollover,
		deleteTodo,
		reorderTodos,
	};
}
