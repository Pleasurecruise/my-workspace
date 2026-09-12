<script lang="ts">
	import { CalendarDays, Check, ChevronLeft, ChevronRight } from "@lucide/svelte";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import type { CommandResponse, Habit, TodoList } from "../../consumer";

	let { todayDate, selectedDate, habits = [], todos = null, onselect }: { todayDate: string; selectedDate: string; habits?: Habit[]; todos?: TodoList | null; onselect: (date: string) => void | Promise<void> } = $props();
	const id = $props.id();
	const monthFormatter = new Intl.DateTimeFormat("en-US", { month: "long", timeZone: "UTC", year: "numeric" });
	const weekdays = ["S", "M", "T", "W", "T", "F", "S"];
	let monthOffset = $state(0);
	$effect(() => { if (selectedDate) monthOffset = 0; });
	let calendar = $derived.by(() => {
		const selected = new Date(`${selectedDate}T00:00:00Z`);
		const first = new Date(selected);
		first.setUTCDate(1);
		first.setUTCMonth(first.getUTCMonth() + monthOffset);
		const last = new Date(first);
		last.setUTCMonth(last.getUTCMonth() + 1, 0);
		return {
			label: monthFormatter.format(first),
			date: first.toISOString().slice(0, 10),
			leadingDays: first.getUTCDay(),
			canGoBack: first.getUTCFullYear() > 1 || first.getUTCMonth() > 0,
			canGoForward: first.getUTCFullYear() < 9999 || first.getUTCMonth() < 11,
			days: Array.from({ length: last.getUTCDate() }, (_, index) => {
				const day = new Date(first);
				day.setUTCDate(index + 1);
				return { date: day.toISOString().slice(0, 10), number: index + 1 };
			}),
		};
	});
	let completed = $state<string[]>([]);
	let error = $state<string | null>(null);
	let liveError = $state<string | null>(null);
	$effect(() => {
		const date = calendar.date;
		const ids = habits.map((habit) => habit.id);
		// Local writes replace this projection; other windows notify through events.
		void todos;
		void todayDate;
		let active = true;
		let revision = 0;
		completed = [];
		error = null;
		liveError = null;
		async function refresh(clear = false) {
			if (!active) return;
			const version = ++revision;
			if (clear) completed = [];
			const response = await invoke<CommandResponse<string[]>>("read_planner_days", { ids, date }).catch((): CommandResponse<string[]> => ({ status: "failed", message: "Could not read calendar completion." }));
			if (!active || version !== revision) return;
			if (response.status === "ready") {
				completed = response.data;
				error = null;
			} else error = response.message;
		}
		const listeners = Promise.allSettled([
			listen<string>("todo-updated", ({ payload }) => { if (payload.slice(0, 7) === date.slice(0, 7)) void refresh(true); }),
			listen<string>("check-in-updated", ({ payload }) => { if (ids.includes(payload)) void refresh(true); }),
			listen<string>("planner-date-changed", () => void refresh(true)),
		]);
		void listeners.then((results) => {
			if (!active) return;
			if (results.some((result) => result.status === "rejected")) liveError = "Live calendar updates are unavailable. Completion refreshes every minute and on focus.";
			void refresh();
		});
		const timer = setInterval(() => void refresh(), 60_000);
		const focus = () => void refresh();
		window.addEventListener("focus", focus);
		return () => { active = false; ++revision; clearInterval(timer); window.removeEventListener("focus", focus); void listeners.then((stops) => stops.forEach((stop) => { if (stop.status === "fulfilled") stop.value(); })); };
	});
</script>

	<section class="calendar-panel" aria-label="Calendar">
	<header><div><CalendarDays size={15} /><h2>Calendar</h2></div><button type="button" disabled={selectedDate === todayDate && monthOffset === 0} onclick={() => { monthOffset = 0; void onselect(todayDate); }}>Today</button></header>
	<div class="month-heading"><button type="button" disabled={!calendar.canGoBack} onclick={() => (monthOffset -= 1)} aria-label="Previous month"><ChevronLeft size={13} /></button><strong>{calendar.label}</strong><button type="button" disabled={!calendar.canGoForward} onclick={() => (monthOffset += 1)} aria-label="Next month"><ChevronRight size={13} /></button></div>
	<div class="month-calendar" aria-label={`${calendar.label} calendar`}>
		{#each weekdays as weekday}<span class="weekday">{weekday}</span>{/each}
		{#each Array(calendar.leadingDays) as _}<span class="calendar-spacer"></span>{/each}
		{#each calendar.days as day}<button type="button" class:today={day.date === todayDate} class:selected={day.date === selectedDate} onclick={() => { monthOffset = 0; void onselect(day.date); }} aria-label={`Select ${day.date}`} aria-pressed={day.date === selectedDate} aria-describedby={completed.includes(day.date) ? `${id}-complete` : undefined} title={completed.includes(day.date) ? "All tasks and check-ins complete" : undefined}>{day.number}{#if completed.includes(day.date)}<span class="completion" aria-hidden="true"><Check size={9} strokeWidth={3} /></span>{/if}</button>{/each}
	</div>
	<span id={`${id}-complete`} class="sr-only">All tasks and check-ins complete</span>
	{#if liveError !== null}<p role="alert">{liveError}</p>{/if}
	{#if error !== null}<p role="alert">{error}</p>{/if}
</section>

<style>
	.calendar-panel { width: 100%; min-width: 0; box-sizing: border-box; padding: 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header,
	header div { display: flex; align-items: center; }
	header { min-height: 1.75rem; justify-content: space-between; gap: 0.5rem; margin-bottom: 0.5rem; }
	header > button { padding: 0 0.3rem; font-size: 0.65rem; }
	header div { gap: 0.4rem; }

	h2 { margin: 0; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	.month-heading { display: grid; grid-template-columns: 1.5rem 1fr 1.5rem; align-items: center; margin: 0.45rem 0; }
	.month-heading strong { font-size: 0.68rem; font-weight: 550; text-align: center; }
	button { display: inline-flex; height: 1.55rem; align-items: center; justify-content: center; padding: 0; border: 1px solid transparent; border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); cursor: pointer; }
	.month-heading button { width: 1.5rem; }
	.month-calendar { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 0.15rem; }
	.month-calendar .weekday { padding: 0.15rem 0; color: var(--color-muted-foreground); font-size: 0.5rem; text-align: center; }
	.month-calendar button { position: relative; width: 100%; font-family: var(--font-mono); font-size: 0.6rem; }
	.month-calendar button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-foreground); }
	.month-calendar button.today { color: var(--color-accent); box-shadow: inset 0 0 0 1px var(--color-accent); }
	.month-calendar button.selected { background: var(--color-accent); color: var(--color-accent-foreground); }
	.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; border: 0; }
	.completion { position: absolute; right: 1px; bottom: 1px; display: flex; color: var(--color-accent); }
	.selected .completion { color: var(--color-accent-foreground); }
	[role="alert"] { margin: 0.4rem 0 0; font-size: 0.65rem; color: var(--color-error); }
	button:disabled { cursor: not-allowed; opacity: 0.45; }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
</style>
