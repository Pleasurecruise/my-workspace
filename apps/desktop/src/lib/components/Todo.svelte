<script lang="ts">
	import { ArrowLeft, CalendarDays, Clock, FileText, ListTodo, MapPin, Plus, Trash2 } from "@lucide/svelte";
	import { tick } from "svelte";
	import type { TodoItem, TodoList } from "../consumer";

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
	let selectedItemId = $state<string | null>(null);
	let backButton = $state<HTMLButtonElement | null>(null);
	let selectedItem = $derived.by((): TodoItem | null => {
		if (todos?.date !== selectedDate) return null;
		const item = todos.items.find((item) => item.id === selectedItemId);
		return item === undefined ? null : item;
	});

	$effect(() => {
		if (selectedItemId !== null && selectedItem === null) selectedItemId = null;
	});

	$effect(() => {
		if (selectedItem === null) return;
		void tick().then(() => backButton?.focus());
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!draft.trim() || loading) return;
		if (await onadd(draft)) draft = "";
	}

	function dateTime(date: string, time: string | null) {
		return time === null ? date : `${date} · ${time}`;
	}
</script>

<section class="todo" aria-labelledby="todo-title">
	{#if selectedItem !== null}
		<div class="todo-detail">
			<header>
				<div class="detail-nav">
					<button bind:this={backButton} type="button" onpointerdown={(event) => event.stopPropagation()} onclick={() => (selectedItemId = null)} aria-label="Back to Todo list"><ArrowLeft size={16} /></button>
					<span>Todo details</span>
					<span class:complete={selectedItem.completed} class="detail-state">{selectedItem.completed ? "Done" : "Open"}</span>
				</div>
				<h2 id="todo-title">{selectedItem.text}</h2>
			</header>
			<div class="todo-detail-scroll">
				<dl>
					<div><dt><CalendarDays size={13} /> Date</dt><dd>{selectedDate}</dd></div>
					{#if selectedItem.details !== null}
						<div><dt><FileText size={13} /> Calendar</dt><dd>{selectedItem.details.calendar}</dd></div>
						<div><dt><Clock size={13} /> Starts</dt><dd>{dateTime(selectedItem.details.startDate, selectedItem.details.startTime)}</dd></div>
						{#if selectedItem.details.endDate !== null}<div><dt><Clock size={13} /> Ends</dt><dd>{dateTime(selectedItem.details.endDate, selectedItem.details.endTime)}</dd></div>{/if}
						{#if selectedItem.details.location !== null}<div><dt><MapPin size={13} /> Location</dt><dd>{selectedItem.details.location}</dd></div>{/if}
					{/if}
				</dl>
				{#if selectedItem.details?.description}
					<section class="todo-description" aria-label="Description"><span>Description</span><p>{selectedItem.details.description}</p></section>
				{:else if selectedItem.details === null}
					<p class="todo-manual">This Todo was added manually.</p>
				{/if}
			</div>
		</div>
	{:else}
		<div class="todo-list-view">
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
							<button class="todo-entry" type="button" aria-label={`View details for ${item.text}`} onpointerdown={(event) => event.stopPropagation()} onclick={() => (selectedItemId = item.id)}>{item.text}</button>
							<button type="button" disabled={loading} onclick={() => void ondelete(item.id)} aria-label={`Delete ${item.text}`}><Trash2 size={13} /></button>
						</li>
					{/each}
				</ul>
			{:else if error !== null}
				<p class="todo-message" role="alert">{error}</p>
			{:else}
				<p class="todo-message">No tasks for this date.</p>
			{/if}
		</div>
	{/if}
</section>

<style>
	.todo { min-width: 0; height: 16rem; padding: 1rem; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.todo-list-view, .todo-detail { display: flex; min-height: 0; height: 100%; flex-direction: column; }
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
	ul { display: grid; min-height: 0; gap: 0.3rem; flex: 1; align-content: start; padding: 0; margin: 0.75rem 0 0; overflow-y: auto; list-style: none; }
	li { min-width: 0; gap: 0.5rem; min-height: 1.8rem; }
	li > input { accent-color: var(--color-accent); }
	li > .todo-entry { min-width: 0; width: auto; height: 1.7rem; flex: 1; justify-content: flex-start; overflow: hidden; padding: 0 0.15rem; border-color: transparent; color: var(--color-foreground); font-size: 0.7rem; text-overflow: ellipsis; white-space: nowrap; }
	li > .todo-entry:hover, li > .todo-entry:focus-visible { background: var(--color-muted); color: var(--color-foreground); }
	li > button { width: 1.7rem; height: 1.7rem; border-color: transparent; opacity: 0; }
	li > .todo-entry, li:hover > button, li > button:focus-visible { opacity: 1; }
	li.completed > .todo-entry { color: var(--color-muted-foreground); text-decoration: line-through; }
	.todo-message { margin: 0.8rem 0 0; color: var(--color-muted-foreground); font-size: 0.7rem; line-height: 1.45; }
	.todo-detail header { padding-bottom: 0.75rem; border-bottom: 1px solid var(--color-divider); }
	.todo-detail .detail-nav { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 0.5rem; min-width: 0; }
	.todo-detail header span, .todo-description > span { color: var(--color-accent); font-size: 0.62rem; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; }
	.todo-detail h2 { display: -webkit-box; margin: 0.55rem 0 0; overflow: hidden; color: var(--color-foreground); font-family: var(--font-serif); font-size: 0.9rem; font-weight: 500; line-height: 1.35; text-transform: none; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; }
	.todo-detail header button { width: 1.7rem; height: 1.7rem; border-color: transparent; border-radius: var(--radius-full); }
	.todo-detail header button:hover { background: var(--color-muted); color: var(--color-foreground); }
	.todo-detail .detail-state { display: inline-flex; padding: 0.2rem 0.4rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-accent) 12%, transparent); color: var(--color-accent); font-size: 0.55rem; letter-spacing: 0; text-transform: none; }
	.todo-detail .detail-state.complete { background: var(--color-muted); color: var(--color-muted-foreground); }
	.todo-detail-scroll { min-height: 0; flex: 1; overflow-y: auto; }
	.todo-detail dl { margin: 0; }
	.todo-detail dl > div { display: grid; grid-template-columns: 5rem minmax(0, 1fr); gap: 0.5rem; padding: 0.6rem 0; border-bottom: 1px solid var(--color-divider); font-size: 0.68rem; }
	.todo-detail dt { display: flex; align-items: center; gap: 0.4rem; color: var(--color-muted-foreground); }
	.todo-detail dd { min-width: 0; margin: 0; overflow-wrap: anywhere; }
	.todo-description { margin-top: 0.8rem; }
	.todo-description p, .todo-manual { margin: 0.45rem 0 0; color: var(--color-muted-foreground); font-size: 0.68rem; line-height: 1.6; white-space: pre-wrap; }
</style>
