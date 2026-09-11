import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMount } from "svelte";
import type { CommandResponse, ExpenseSnapshot } from "../../consumer";

export function createLedgerSession() {
	let data = $state<ExpenseSnapshot | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let writing = $state(false);
	let date = "";
	let context = 0;
	let revision = 0;
	let queued = false;
	let disposed = false;
	let writeError: string | null = null;

	async function load(selected = date) {
		if (disposed || !selected) return;
		if (selected !== date) {
			++context;
			++revision;
			date = selected;
			data = null;
			error = null;
			writeError = null;
		}
		loading = true;
		if (writing) {
			queued = true;
			return;
		}
		const version = ++revision;
		const response = await invoke<CommandResponse<ExpenseSnapshot>>("read_expenses", { date });
		if (disposed || version !== revision) return;
		loading = false;
		if (response.status === "ready") {
			data = response.data;
			error = writeError;
		} else error = writeError === null ? response.message : `${writeError} · ${response.message}`;
	}

	async function write(operation: () => Promise<CommandResponse<ExpenseSnapshot>>) {
		const generation = context;
		++revision;
		writing = true;
		loading = true;
		writeError = null;
		error = null;
		const response = await operation();
		writing = false;
		if (disposed) return false;
		loading = false;
		const current = generation === context;
		if (current) {
			if (response.status === "ready") data = response.data;
			else {
				writeError = response.message;
				error = response.message;
			}
		}
		if (queued || !current) {
			queued = false;
			void load();
		}
		return current && response.status === "ready";
	}

	async function save(
		id: string | null,
		amount: string,
		category: string,
		description: string | null,
	) {
		if (disposed || loading || writing || !date) return false;
		const selected = date;
		return write(() =>
			id === null
				? invoke<CommandResponse<ExpenseSnapshot>>("create_expense", {
						date: selected,
						amount,
						category,
						description,
					})
				: invoke<CommandResponse<ExpenseSnapshot>>("update_expense", {
						date: selected,
						id,
						amount,
						category,
						description,
					}),
		);
	}

	async function remove(id: string) {
		if (disposed || loading || writing || !date) return false;
		const selected = date;
		return write(() =>
			invoke<CommandResponse<ExpenseSnapshot>>("delete_expense", { date: selected, id }),
		);
	}

	onMount(() => {
		const listener = listen<string>("expenses-updated", ({ payload }) => {
			if (payload.slice(0, 7) === date.slice(0, 7)) void load();
		});
		void listener.then(() => {
			if (!disposed) void load();
		});
		const focus = () => void load();
		window.addEventListener("focus", focus);
		const timer = window.setInterval(() => void load(), 60_000);
		return () => {
			disposed = true;
			++revision;
			window.clearInterval(timer);
			window.removeEventListener("focus", focus);
			void listener.then((stop) => stop());
		};
	});

	return {
		get data() {
			return data;
		},
		get error() {
			return error;
		},
		get loading() {
			return loading;
		},
		get writing() {
			return writing;
		},
		load,
		save,
		remove,
	};
}
