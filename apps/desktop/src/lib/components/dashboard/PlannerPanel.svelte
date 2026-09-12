<script lang="ts">
	import { untrack } from "svelte";
	import type { Habit } from "../../consumer";
	import type { createDashboardSession } from "./session.svelte";
	import CalendarPanel from "./CalendarPanel.svelte";
	import Todo from "./Todo.svelte";
	import HabitsPanel from "./HabitsPanel.svelte";
	let { session, habits, embedded = false, onchange = null }: {
		session: Pick<ReturnType<typeof createDashboardSession>, "todayDate" | "selectedDate" | "selectDate" | "todos" | "loadTodos" | "addTodo" | "editTodo" | "toggleTodo" | "deleteTodo" | "reorderTodos" | "setTodoRollover">;
		habits: Habit[];
		embedded?: boolean;
		onchange?: ((habits: Habit[]) => Promise<boolean>) | null;
	} = $props();
	const todayDate = $derived(session.todayDate);
	const selectedDate = $derived(session.selectedDate);
	$effect(() => {
		if (!selectedDate) return;
		untrack(() => { void session.loadTodos(); });
	});
</script>

<section class="planner" class:compact={embedded} aria-label="Daily Planner">
	<div class="planner-calendar"><CalendarPanel {todayDate} {selectedDate} {habits} todos={session.todos.data} onselect={session.selectDate} /></div>
	<div class="planner-todos"><Todo {embedded} todos={session.todos.data?.date === selectedDate ? session.todos.data : null} error={session.todos.error} loading={session.todos.loading} {selectedDate} onadd={session.addTodo} onedit={session.editTodo} ontoggle={session.toggleTodo} ondelete={session.deleteTodo} onreorder={session.reorderTodos} onrollover={session.setTodoRollover} /></div>
	<HabitsPanel {selectedDate} habits={habits} onchange={onchange} />
</section>

<style>
	.planner { display: grid; grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr) minmax(0, 1.2fr); overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.planner > div { min-width: 0; border-right: 1px solid var(--color-border); }
	.planner > div > :global(section) { border: 0; box-shadow: none; border-radius: 0; margin: 0; }
	.planner.compact { grid-template-columns: 1fr; max-height: 32rem; overflow-y: auto; }
	.planner.compact > div { border-right: 0; border-bottom: 1px solid var(--color-border); }

</style>
