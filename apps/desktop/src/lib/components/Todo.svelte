<script lang="ts">
	import { CalendarCheck, ChevronLeft, ChevronRight, Plus, Trash2 } from "@lucide/svelte";
	import type { TodoList } from "../consumer";

	let {
		todos,
		error,
		loading,
		todayDate,
		selectedDate,
		onadd,
		ontoggle,
		ondelete,
		onselect,
	}: {
		todos: TodoList | null;
		error: string | null;
		loading: boolean;
		todayDate: string;
		selectedDate: string;
		onadd: (text: string) => Promise<boolean>;
		ontoggle: (id: string, completed: boolean) => Promise<void>;
		ondelete: (id: string) => Promise<void>;
		onselect: (date: string) => Promise<void>;
	} = $props();

	const monthFormatter = new Intl.DateTimeFormat("zh-CN", {
		month: "long",
		timeZone: "UTC",
		year: "numeric",
	});
	const weekdays = ["一", "二", "三", "四", "五", "六", "日"];
	let draft = $state("");
	let monthOffset = $state(0);
	let calendar = $derived.by(() => {
		const selected = new Date(`${selectedDate}T00:00:00Z`);
		const first = new Date(Date.UTC(selected.getUTCFullYear(), selected.getUTCMonth() + monthOffset, 1));
		const dayCount = new Date(Date.UTC(first.getUTCFullYear(), first.getUTCMonth() + 1, 0)).getUTCDate();
		return {
			label: monthFormatter.format(first),
			leadingDays: (first.getUTCDay() + 6) % 7,
			days: Array.from({ length: dayCount }, (_, index) => {
				const day = new Date(Date.UTC(first.getUTCFullYear(), first.getUTCMonth(), index + 1));
				return { date: day.toISOString().slice(0, 10), number: index + 1 };
			}),
		};
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!draft.trim() || loading) return;
		if (await onadd(draft)) draft = "";
	}
</script>

<section class="todo" aria-labelledby="todo-title">
	<div class="todo-heading">
		<div><CalendarCheck size={15} /><h2 id="todo-title">Todo</h2></div>
		{#if todos?.date === selectedDate}<span>{todos.items.filter((item) => item.completed).length}/{todos.items.length}</span>{/if}
	</div>

	<div class="month-heading">
		<button type="button" onclick={() => (monthOffset -= 1)} aria-label="上个月"><ChevronLeft size={13} /></button>
		<strong>{calendar.label}</strong>
		<button type="button" onclick={() => (monthOffset += 1)} aria-label="下个月"><ChevronRight size={13} /></button>
	</div>
	<div class="month-calendar" aria-label={`${calendar.label}日历`}>
		{#each weekdays as weekday}<span class="weekday">{weekday}</span>{/each}
		{#each Array(calendar.leadingDays) as _}<span class="calendar-spacer"></span>{/each}
		{#each calendar.days as day}
			<button
				type="button"
				class:today={day.date === todayDate}
				class:selected={day.date === selectedDate}
				disabled={loading}
				onclick={() => {
					monthOffset = 0;
					void onselect(day.date);
				}}
				aria-label={`选择 ${day.date}`}
				aria-pressed={day.date === selectedDate}
			>{day.number}</button>
		{/each}
	</div>

	<form onsubmit={submit}>
		<input bind:value={draft} maxlength="120" placeholder="添加这一天要做的事" aria-label={`${selectedDate} 的新 Todo`} />
		<button type="submit" disabled={loading || !draft.trim()} aria-label="添加 Todo"><Plus size={14} /></button>
	</form>

	{#if error !== null && todos?.date !== selectedDate}
		<p class="todo-message" role="alert">{error}</p>
	{:else if loading && todos?.date !== selectedDate}
		<p class="todo-message">正在读取 {selectedDate} 的 Todo…</p>
	{:else if todos?.date === selectedDate && todos.items.length > 0}
		<ul>
			{#each todos.items as item (item.id)}
				<li class:completed={item.completed}>
					<input
						type="checkbox"
						checked={item.completed}
						disabled={loading}
						onchange={(event) => void ontoggle(item.id, event.currentTarget.checked)}
						aria-label={`标记 ${item.text} 为${item.completed ? "未完成" : "已完成"}`}
					/>
					<span>{item.text}</span>
					<button type="button" disabled={loading} onclick={() => void ondelete(item.id)} aria-label={`删除 ${item.text}`}><Trash2 size={13} /></button>
				</li>
			{/each}
		</ul>
	{:else if error !== null}
		<p class="todo-message" role="alert">{error}</p>
	{:else}
		<p class="todo-message">这一天还没有事项。</p>
	{/if}
</section>

<style>
	.todo { min-width: 0; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.todo-heading, .todo-heading div, form, li { display: flex; align-items: center; }
	.todo-heading { justify-content: space-between; gap: 0.5rem; margin-bottom: 0.9rem; }
	.todo-heading div { gap: 0.4rem; }
	h2 { margin: 0; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	.todo-heading > span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.65rem; }
	.month-heading { display: grid; grid-template-columns: 1.5rem 1fr 1.5rem; align-items: center; margin-bottom: 0.45rem; }
	.month-heading strong { color: var(--color-foreground); font-size: 0.68rem; font-weight: 550; text-align: center; }
	.month-heading button { width: 1.5rem; height: 1.5rem; border-color: transparent; }
	.month-calendar { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 0.15rem; margin-bottom: 0.8rem; }
	.month-calendar .weekday { padding: 0.15rem 0; color: var(--color-muted-foreground); font-size: 0.5rem; text-align: center; }
	.month-calendar button { width: 100%; height: 1.55rem; border-color: transparent; font-family: var(--font-mono); font-size: 0.6rem; }
	.month-calendar button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-foreground); }
	.month-calendar button.today { color: var(--color-accent); box-shadow: inset 0 0 0 1px var(--color-accent); }
	.month-calendar button.selected { background: var(--color-accent); color: var(--color-accent-foreground); }
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
