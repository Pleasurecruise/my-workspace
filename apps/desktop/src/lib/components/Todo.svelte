<script lang="ts">
	import { ListTodo, Plus, Trash2 } from "@lucide/svelte";
	import type { TodoList } from "../consumer";

	let {
		todos,
		error,
		loading,
		selectedDate,
		onadd,
		ontoggle,
		ondelete,
	}: {
		todos: TodoList | null;
		error: string | null;
		loading: boolean;
		selectedDate: string;
		onadd: (text: string) => Promise<boolean>;
		ontoggle: (id: string, completed: boolean) => Promise<void>;
		ondelete: (id: string) => Promise<void>;
	} = $props();
	let draft = $state("");

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!draft.trim() || loading) return;
		if (await onadd(draft)) draft = "";
	}
</script>

<section class="todo" aria-labelledby="todo-title">
	<div class="todo-heading">
		<div><ListTodo size={15} /><h2 id="todo-title">Todo</h2></div>
		{#if todos?.date === selectedDate}<span>{todos.items.filter((item) => item.completed).length}/{todos.items.length}</span>{/if}
	</div>

	<form onsubmit={submit}>
		<input bind:value={draft} maxlength="120" placeholder="Add a task for this date" aria-label={`New Todo for ${selectedDate}`} />
		<button type="submit" disabled={loading || !draft.trim()} aria-label="Add Todo"><Plus size={14} /></button>
	</form>

	{#if error !== null && todos?.date !== selectedDate}
		<p class="todo-message" role="alert">{error}</p>
	{:else if loading && todos?.date !== selectedDate}
		<p class="todo-message">Loading Todos for {selectedDate}…</p>
	{:else if todos?.date === selectedDate && todos.items.length > 0}
		<ul>
			{#each todos.items as item (item.id)}
				<li class:completed={item.completed}>
					<input
						type="checkbox"
						checked={item.completed}
						disabled={loading}
						onchange={(event) => void ontoggle(item.id, event.currentTarget.checked)}
						aria-label={`Mark ${item.text} as ${item.completed ? "incomplete" : "complete"}`}
					/>
					<span>{item.text}</span>
					<button type="button" disabled={loading} onclick={() => void ondelete(item.id)} aria-label={`Delete ${item.text}`}><Trash2 size={13} /></button>
				</li>
			{/each}
		</ul>
	{:else if error !== null}
		<p class="todo-message" role="alert">{error}</p>
	{:else}
		<p class="todo-message">No tasks for this date.</p>
	{/if}
</section>

<style>
	.todo { min-width: 0; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.todo-heading, .todo-heading div, form, li { display: flex; align-items: center; }
	.todo-heading { justify-content: space-between; gap: 0.5rem; margin-bottom: 0.9rem; }
	.todo-heading div { gap: 0.4rem; }
	h2 { margin: 0; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	.todo-heading > span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.65rem; }
	form { gap: 0.4rem; }
	form input { min-width: 0; height: 2rem; flex: 1; padding: 0 0.65rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font-size: 0.72rem; outline: none; }
	form input:focus { border-color: var(--color-accent); }
	button { display: inline-flex; width: 2rem; height: 2rem; flex: 0 0 auto; align-items: center; justify-content: center; padding: 0; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); cursor: pointer; }
	button:disabled { cursor: not-allowed; opacity: 0.45; }
	ul { display: grid; gap: 0.3rem; max-height: 12rem; padding: 0; margin: 0.75rem 0 0; overflow-y: auto; list-style: none; }
	li { min-width: 0; gap: 0.5rem; min-height: 1.8rem; }
	li > input { accent-color: var(--color-accent); }
	li > span { min-width: 0; flex: 1; overflow: hidden; font-size: 0.7rem; text-overflow: ellipsis; white-space: nowrap; }
	li > button { width: 1.7rem; height: 1.7rem; border-color: transparent; opacity: 0; }
	li:hover > button, li > button:focus-visible { opacity: 1; }
	li.completed > span { color: var(--color-muted-foreground); text-decoration: line-through; }
	.todo-message { margin: 0.8rem 0 0; color: var(--color-muted-foreground); font-size: 0.7rem; line-height: 1.45; }
</style>
