<script lang="ts">
	import { ArrowLeft, CalendarDays, Clock, FileText, ListTodo, MapPin, Pencil, Plus, Trash2 } from "@lucide/svelte";
	import { tick } from "svelte";
	import type { TodoItem, TodoList } from "../../consumer";

	let {
		todos,
		error,
		loading,
		selectedDate,
		onadd,
		onedit,
		ontoggle,
		ondelete,
		embedded = false,
	}: {
		todos: TodoList | null;
		error: string | null;
		loading: boolean;
		selectedDate: string;
		onadd: (text: string, description: string) => Promise<boolean>;
		onedit: (id: string, text: string, description: string) => Promise<boolean>;
		ontoggle: (id: string, completed: boolean) => Promise<void>;
		ondelete: (id: string) => Promise<void>;
		embedded?: boolean;
	} = $props();
	const headingId = $props.id();
	let draft = $state("");
	let description = $state("");
	let showDescription = $state(false);
	let editing = $state(false);
	let editText = $state("");
	let editDescription = $state("");
	let saving = $state(false);
	let selectedItemId = $state<string | null>(null);
	let backButton = $state<HTMLButtonElement | null>(null);
	let selectedItem = $derived.by((): TodoItem | null => {
		if (todos?.date !== selectedDate) return null;
		const item = todos.items.find((item) => item.id === selectedItemId);
		return item === undefined ? null : item;
	});

	$effect(() => {
		if (selectedItem === null) { selectedItemId = null; editing = false; }
	});

	$effect(() => {
		if (selectedItem === null) return;
		void tick().then(() => backButton?.focus());
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!draft.trim() || loading || saving) return;
		const submitted = { text: draft, description, date: selectedDate };
		saving = true;
		const saved = await onadd(submitted.text, submitted.description);
		saving = false;
		if (saved && selectedDate === submitted.date && draft === submitted.text && description === submitted.description) {
			draft = ""; description = ""; showDescription = false;
		}
	}

	function startEditing() {
		if (selectedItem === null) return;
		editText = selectedItem.text;
		editDescription = selectedItem.description === null ? "" : selectedItem.description;
		editing = true;
	}

	async function saveEdit(event: SubmitEvent) {
		event.preventDefault();
		if (selectedItem === null || !editText.trim() || loading || saving) return;
		const id = selectedItem.id;
		const date = selectedDate;
		saving = true;
		const saved = await onedit(id, editText, editDescription);
		saving = false;
		if (saved && selectedItemId === id && selectedDate === date) editing = false;
	}

	function formatDateTime(date: string, time: string | null) {
		return time === null ? date : `${date} · ${time}`;
	}
</script>

<section class="todo" class:embedded aria-labelledby={headingId}>
	{#if error !== null}<p class="todo-message" role="alert">{error}</p>{/if}
	{#if selectedItem !== null}
		<div class="todo-detail">
			<header>
				<div class="detail-nav">
					<button bind:this={backButton} type="button" onpointerdown={(event) => event.stopPropagation()} disabled={saving} onclick={() => { selectedItemId = null; editing = false; }} aria-label="Back to Todo list"><ArrowLeft size={16} /></button>
					<span>Todo details</span>
					<span class:complete={selectedItem.completed} class="detail-state">{selectedItem.completed ? "Done" : "Open"}</span>
				</div>
				<div class="detail-title">
					<h2 id={headingId}>{selectedItem.text}</h2>
					{#if selectedItem.details === null && !editing}<button class="edit-button" type="button" aria-label="Edit Todo" title="Edit Todo" disabled={loading || saving} onpointerdown={(event) => event.stopPropagation()} onclick={startEditing}><Pencil size={14} /></button>{/if}
				</div>
			</header>
			<div class="todo-detail-scroll">
				{#if editing}
					<form class="edit-form" onsubmit={saveEdit}>
						<label>Title<input bind:value={editText} maxlength="120" required disabled={saving} /></label>
						<label>Description<textarea bind:value={editDescription} maxlength="4000" rows="3" disabled={saving}></textarea></label>
						<div class="edit-actions"><button type="button" disabled={saving} onclick={() => (editing = false)}>Cancel</button><button type="submit" disabled={loading || saving || !editText.trim()}>{saving ? "Saving…" : "Save changes"}</button></div>
					</form>
				{:else}
					{#if selectedItem.details !== null}<p class="todo-manual">Edit this task in its source calendar.</p>{/if}
				<dl>
					<div><dt><CalendarDays size={13} /> Date</dt><dd>{selectedDate}</dd></div>
					{#if selectedItem.details !== null}
						<div><dt><FileText size={13} /> Calendar</dt><dd>{selectedItem.details.calendar}</dd></div>
						<div><dt><Clock size={13} /> Starts</dt><dd>{formatDateTime(selectedItem.details.startDate, selectedItem.details.startTime)}</dd></div>
						{#if selectedItem.details.endDate !== null}<div><dt><Clock size={13} /> Ends</dt><dd>{formatDateTime(selectedItem.details.endDate, selectedItem.details.endTime)}</dd></div>{/if}
						{#if selectedItem.details.location !== null}<div><dt><MapPin size={13} /> Location</dt><dd>{selectedItem.details.location}</dd></div>{/if}
					{/if}
				</dl>
				{#if selectedItem.description}
					<section class="todo-description" aria-label="Description"><span>Description</span><p>{selectedItem.description}</p></section>
				{:else if selectedItem.details === null}
					<p class="todo-manual">This Todo was added manually.</p>
				{/if}
				{/if}
			</div>
		</div>
	{:else}
		<div class="todo-list-view">
			<div class="todo-heading">
				<div><ListTodo size={15} /><h2 id={headingId}>Todo</h2></div>
				{#if todos?.date === selectedDate}<span>{todos.items.filter((item) => item.completed).length}/{todos.items.length}</span>{/if}
			</div>

			<form class="add-form" onsubmit={submit}>
				<div class="add-row">
				<input disabled={saving} bind:value={draft} maxlength="120" placeholder="Add a task for this date" aria-label={`New Todo for ${selectedDate}`} />
				<button type="submit" disabled={loading || saving || !draft.trim()} aria-label="Add Todo"><Plus size={14} /></button>
				<button type="button" aria-label="Add description" title={showDescription ? "Hide description" : "Add description"} aria-expanded={showDescription} disabled={saving} onclick={() => (showDescription = !showDescription)}><FileText size={14} /></button>
				</div>
				{#if showDescription}<textarea bind:value={description} maxlength="4000" rows="2" disabled={saving} placeholder="Add a description (optional)" aria-label="New Todo description"></textarea>{/if}
			</form>
			{#if loading && todos?.date !== selectedDate}
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
			{:else if error === null}
				<p class="todo-message">No tasks for this date.</p>
			{/if}
		</div>
	{/if}
</section>

<style>
	.todo { display: flex; flex-direction: column; gap: 0.3rem; min-width: 0; height: 16rem; padding: 0.75rem; overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.todo-list-view, .todo-detail { flex: 1; display: flex; min-height: 0; height: 100%; flex-direction: column; }
	.todo.embedded { height: 14rem; }
	.todo.embedded .todo-heading { display: none; }
	.todo.embedded li { min-height: 2.2rem; }
	.todo.embedded li > .todo-entry { font-size: 0.8rem; }
	.todo.embedded li > input { appearance: none; width: 16px; height: 16px; flex: 0 0 16px; margin: 0; border: 1px solid var(--color-muted-foreground); border-radius: var(--radius-full); background: transparent; cursor: pointer; }
	.todo.embedded li > input:checked { border-color: var(--color-accent); background: var(--color-accent); }
	.todo.embedded li > input:checked::after { content: ""; display: block; width: 7px; height: 4px; margin: 3px 3px; border-left: 1.5px solid var(--color-accent-foreground); border-bottom: 1.5px solid var(--color-accent-foreground); transform: rotate(-45deg); }
	.todo.embedded li > input:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 3px; }
	.todo-heading, .todo-heading div, form, li { display: flex; align-items: center; }
	.todo-heading { justify-content: space-between; gap: 0.5rem; margin-bottom: 0.5rem; }
	.todo-heading div { gap: 0.4rem; }

	h2 { margin: 0; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	.todo-heading > span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.65rem; }
	form { gap: 0.4rem; }
	form input { min-width: 0; height: 2rem; flex: 1; padding: 0 0.65rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font-size: 0.72rem; outline: none; }
	form input:focus { border-color: var(--color-accent); }
	button { display: inline-flex; width: 2rem; height: 2rem; flex: 0 0 auto; align-items: center; justify-content: center; padding: 0; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); cursor: pointer; }
	button:disabled { cursor: not-allowed; opacity: 0.45; }
	ul { display: grid; min-height: 0; gap: 0.15rem; flex: 1; align-content: start; padding: 0; margin: 0.5rem 0 0; overflow-y: auto; list-style: none; }
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

	.detail-title { display: flex; align-items: center; gap: 0.4rem; min-width: 0; margin-top: 0.55rem; }
	.todo-detail h2 { display: -webkit-box; min-width: 0; margin: 0; overflow: hidden; color: var(--color-foreground); font-family: var(--font-serif); font-size: 0.9rem; font-weight: 500; line-height: 1.35; text-transform: none; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; }
	.todo-detail header button { width: 1.7rem; height: 1.7rem; border-color: transparent; border-radius: var(--radius-full); }
	.todo-detail header button:hover, .todo-detail header button:focus-visible { background: var(--color-muted); color: var(--color-foreground); }
	.todo-detail .detail-state { display: inline-flex; padding: 0.2rem 0.4rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-accent) 12%, transparent); color: var(--color-accent); font-size: 0.55rem; letter-spacing: 0; text-transform: none; }
	.todo-detail .detail-state.complete { background: var(--color-muted); color: var(--color-muted-foreground); }
	.todo-detail-scroll { min-height: 0; flex: 1; overflow-y: auto; }
	.todo-detail dl { margin: 0; }
	.todo-detail dl > div { display: grid; grid-template-columns: 5rem minmax(0, 1fr); gap: 0.5rem; padding: 0.6rem 0; border-bottom: 1px solid var(--color-divider); font-size: 0.68rem; }
	.todo-detail dt { display: flex; align-items: center; gap: 0.4rem; color: var(--color-muted-foreground); }
	.todo-detail dd { min-width: 0; margin: 0; overflow-wrap: anywhere; }
	.todo-description { margin-top: 0.8rem; }
	.todo-description p, .todo-manual { margin: 0.45rem 0 0; color: var(--color-muted-foreground); font-size: 0.68rem; line-height: 1.6; white-space: pre-wrap; }

	.add-form, .edit-form { flex-direction: column; align-items: stretch; }
	.add-row { display: flex; gap: 0.4rem; }
	textarea { box-sizing: border-box; width: 100%; resize: vertical; min-height: 3rem; max-height: 8rem; padding: 0.5rem 0.65rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font: inherit; font-size: 0.72rem; }
	textarea:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 1px; }
	.edit-form { padding-top: 0.5rem; gap: 0.5rem; }
	.edit-form label { display: flex; flex-direction: column; gap: 0.25rem; color: var(--color-muted-foreground); font-size: 0.68rem; }
	.edit-form input { flex: auto; }
	.edit-actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
	.edit-actions button { width: auto; padding: 0 0.65rem; gap: 0.4rem; font-size: 0.68rem; }
</style>
