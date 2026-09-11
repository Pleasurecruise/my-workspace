<script lang="ts">
	import { Check, ListChecks, Plus, Settings2, Trash2, Undo2 } from "@lucide/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import type { CheckIn, CommandResponse, Habit } from "../../consumer";
	let { habits, selectedDate, onchange = null }: { habits: Habit[]; selectedDate: string; onchange?: ((habits: Habit[]) => Promise<boolean>) | null } = $props();
	let progress = $state<CheckIn[]>([]);
	let error = $state<string | null>(null);
	let writeError = $state<string | null>(null);
	let busy = $state(false);
	let names = $state("");
	let managing = $state(false);
	let saving = $state(false);
	let revision = 0;
	let context = 0;
	let disposed = true;
	async function refresh() {
		if (busy || disposed) return;
		const version = ++revision;
		const date = selectedDate;
		const ids = habits.map((habit) => habit.id);
		const response = await invoke<CommandResponse<CheckIn[]>>("read_check_ins", { ids, date });
		if (version !== revision || disposed) return;
		if (response.status === "ready") {
			progress = response.data;
			error = null;
		} else error = response.message;
	}
	async function set(id: string, completed: boolean) {
		const state = progress.find((item) => item.id === id && item.date === selectedDate);
		if (busy || !state?.editable) return;
		const date = selectedDate;
		const generation = context;
		++revision;
		busy = true;
		writeError = null;
		const response = await invoke<CommandResponse<CheckIn>>("set_check_in", { id, date, completed });
		busy = false;
		if (disposed) return;
		if (generation === context) {
			if (response.status === "failed") writeError = response.message;
			else progress = progress.map((item) => item.id === id ? response.data : item);
		}
		// A date change or cross-window update during a write always rereads the current day.
		await refresh();
	}
	async function add() {
		if (onchange === null || saving) return;
		const submitted = names;
		const additions = submitted.split(/[,，\n]/).map((name) => name.trim()).filter(Boolean);
		if (additions.length === 0) return;
		saving = true;
		if (await onchange([...habits, ...additions.map((name) => ({ id: `habit-${crypto.randomUUID()}`, name }))])) { if (names === submitted) names = ""; }
		saving = false;
	}
	$effect(() => {
		const ids = habits.map((habit) => habit.id);
		const date = selectedDate;
		++context;
		disposed = false;
		progress = [];
		error = null;
		writeError = null;
		let active = true;
		const listener = listen<string>("check-in-updated", ({ payload }) => {
			if (active && ids.includes(payload)) void refresh();
		});
		const rollover = listen<string>("planner-date-changed", () => {
			if (active) void refresh();
		});
		void listener.then(() => { if (active && date === selectedDate) void refresh(); });
		const timer = setInterval(() => void refresh(), 60_000);
		const focus = () => void refresh();
		window.addEventListener("focus", focus);
		return () => { active = false; disposed = true; ++revision; clearInterval(timer); window.removeEventListener("focus", focus); void listener.then((stop) => stop()); void rollover.then((stop) => stop()); };
	});
</script>
<section class="habits" aria-label="Daily habits">
	<header><div class="heading"><ListChecks size={15} /><h2>Daily check-in</h2></div>{#if onchange !== null}<button type="button" aria-label="Manage habits" title="Manage habits" aria-pressed={managing} onclick={() => managing = !managing}><Settings2 size={15} /></button>{/if}</header>
	<p class="subtitle">{selectedDate}</p>
	{#if managing && onchange !== null}
		<form onsubmit={(event) => { event.preventDefault(); void add(); }}><label>New habits<textarea aria-label="New habits" placeholder="Read, Exercise, Drink water" bind:value={names}></textarea></label><button aria-label="Add habits" title="Add habits" disabled={saving || busy || !names.trim()}><Plus size={15} /></button></form>
	{/if}
	<div class="rows">
		{#each habits as habit (habit.id)}
			{@const state = progress.find((item) => item.id === habit.id && item.date === selectedDate)}
			<div class="habit">
				<div class="habit-info"><strong>{habit.name}</strong>{#if state}<small>{state.streak} day streak · {state.total} total</small><div class="history" aria-label={`${habit.name}: last 28 days`}>{#each state.days as day}<span class:done={day.completed} role="img" aria-label={`${day.date}: ${day.completed ? "Checked in" : "Not checked in"}`} title={`${day.date}: ${day.completed ? "Checked in" : "Not checked in"}`}></span>{/each}</div>{/if}</div>
				<button type="button" disabled={busy || !state?.editable} aria-label={`${state?.completed ? "Undo" : "Check in"} ${habit.name}`} title={`${state?.completed ? "Undo" : "Check in"} ${habit.name}`} aria-pressed={state?.completed === true} onclick={() => void set(habit.id, !state?.completed)}>{#if state?.completed}<Undo2 size={15} />{:else}<Check size={15} />{/if}</button>
				{#if managing && onchange !== null}<button type="button" disabled={saving || busy} aria-label={`Remove ${habit.name}`} title={`Remove ${habit.name}`} onclick={async () => { if (onchange === null) return; saving = true; await onchange(habits.filter((item) => item.id !== habit.id)); saving = false; }}><Trash2 size={14} /></button>{/if}
			</div>
		{/each}
		{#if progress.some((item) => !item.editable)}<p>Future check-ins are read-only.</p>{/if}
		{#if habits.length === 0}<p>Add the habits you want to track each day.</p>{/if}
	</div>

	{#if writeError !== null || error !== null}<p role="alert">{writeError === null ? error : writeError}</p>{/if}
</section>
<style>
	.habits { display: flex; flex-direction: column; gap: 0.5rem; min-width: 0; height: 16rem; padding: 0.75rem; overflow: hidden; box-sizing: border-box; }
	header, .habit, .heading { display: flex; align-items: center; gap: 0.45rem; }
	header { min-height: 1.75rem; flex: 0 0 auto; justify-content: space-between; }
	.heading { gap: 0.4rem; color: var(--color-muted-foreground); }
	h2 { margin: 0; font-size: 0.72rem; font-weight: 500; color: var(--color-muted-foreground); text-transform: uppercase; }
	p, small, label { font-size: 0.68rem; color: var(--color-muted-foreground); }
	.subtitle { margin: 0; }
	.rows { flex: 1; min-height: 0; overflow-y: auto; }
	.habit { min-height: 1.8rem; gap: 0.5rem; padding: 0.4rem 0; }
	.habit-info { display: grid; flex: 1; min-width: 0; gap: 0.2rem; }
	strong { font-size: 0.7rem; font-weight: 400; line-height: 1.4; overflow-wrap: anywhere; }
	small { font-size: 0.65rem; line-height: 1.4; }
	button { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 auto; width: 1.75rem; height: 1.75rem; border: 1px solid transparent; border-radius: var(--radius-md); background: transparent; color: var(--color-muted-foreground); padding: 0; font-size: 0.65rem; cursor: pointer; }
	button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-accent); }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	button:disabled { opacity: 0.5; cursor: default; }
	button[aria-pressed="true"] { color: var(--color-accent); }
	.history { display: grid; grid-template-columns: repeat(28, 1fr); gap: 2px; }
	.history span { height: 4px; border-radius: var(--radius-sm); background: var(--color-muted); }
	.history .done { background: var(--color-accent); }
	form { display: flex; align-items: end; gap: 0.4rem; }
	label { flex: 1; min-width: 0; }
	textarea { display: block; width: 100%; box-sizing: border-box; resize: vertical; min-height: 3rem; max-height: 5rem; padding: 0.5rem 0.65rem; font: inherit; background: var(--color-background); color: var(--color-foreground); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
	[role="alert"] { color: var(--color-error); margin: 0; }
</style>
