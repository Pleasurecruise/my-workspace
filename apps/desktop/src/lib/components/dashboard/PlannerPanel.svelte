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
	let compactView = $state<"tasks" | "calendar" | "habits">("tasks");
	const todayDate = $derived(session.todayDate);
	const selectedDate = $derived(session.selectedDate);
	$effect(() => {
		if (!selectedDate) return;
		untrack(() => { void session.loadTodos(); });
	});
</script>

<section class="planner" class:compact={embedded} aria-label="Daily Planner">
	{#if embedded}
		<div class="compact-toolbar"><nav aria-label="Planner sections">
			<button type="button" aria-pressed={compactView === "tasks"} onclick={() => (compactView = "tasks")}>Tasks</button>
			<button type="button" aria-pressed={compactView === "calendar"} onclick={() => (compactView = "calendar")}>Calendar</button>
			<button type="button" aria-pressed={compactView === "habits"} onclick={() => (compactView = "habits")}>Habits</button>
		</nav><time datetime={selectedDate}>{selectedDate === todayDate ? "Today · " : ""}{selectedDate}</time></div>
	{/if}
	<div class="planner-calendar" hidden={embedded && compactView !== "calendar"}><CalendarPanel {todayDate} {selectedDate} {habits} todos={session.todos.data} onselect={session.selectDate} /></div>
	<div class="planner-todos" hidden={embedded && compactView !== "tasks"}><Todo {embedded} todos={session.todos.data?.date === selectedDate ? session.todos.data : null} error={session.todos.error} loading={session.todos.loading} {selectedDate} onadd={session.addTodo} onedit={session.editTodo} ontoggle={session.toggleTodo} ondelete={session.deleteTodo} onreorder={session.reorderTodos} onrollover={session.setTodoRollover} /></div>
	<div class="planner-habits" hidden={embedded && compactView !== "habits"}><HabitsPanel {selectedDate} habits={habits} onchange={onchange} /></div>
</section>

<style>
	.planner { display: grid; grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr) minmax(0, 1.2fr); overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.planner > div { min-width: 0; border-right: 1px solid var(--color-border); }
	.planner > div > :global(section) { border: 0; box-shadow: none; border-radius: 0; margin: 0; }
	.planner > .planner-habits { border-right: 0; }
	.planner.compact { display: flex; flex-direction: column; overflow: visible; }
	.planner.compact > div { border: 0; }
	.planner [hidden] { display: none; }
	.compact-toolbar { display: flex; align-items: center; flex-wrap: wrap; justify-content: space-between; gap: 8px; margin-bottom: 12px; }
	.compact-toolbar time { color: var(--color-muted-foreground); font-size: 0.68rem; font-variant-numeric: tabular-nums; }
	.compact nav { display: flex; align-self: flex-start; gap: 4px; padding: 3px; border-radius: var(--radius-full); background: var(--color-muted); }
	.compact nav button { padding: 6px 14px; border: 0; border-radius: var(--radius-full); background: transparent; color: var(--color-muted-foreground); font-family: var(--font-sans); font-size: 0.7rem; cursor: pointer; }
	.compact nav button[aria-pressed="true"] { background: var(--color-foreground); color: var(--color-background); }
	.compact nav button:focus-visible { outline: 2px solid var(--color-foreground); outline-offset: 3px; }
	.compact nav button:hover:not([aria-pressed="true"]) { color: var(--color-foreground); }

</style>
