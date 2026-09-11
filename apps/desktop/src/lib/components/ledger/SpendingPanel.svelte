<script lang="ts">
	import { Select } from "@my-workspace/ui";
	import { ChevronLeft, ChevronRight, Check, Pencil, Plus, Trash2, WalletCards, X } from "@lucide/svelte";
	import { untrack } from "svelte";
	import type { ExpenseEntry } from "../../consumer";
	import { createLedgerSession } from "./session.svelte";
	let { selectedDate, todayDate, onselect, embedded = false }: { selectedDate: string; todayDate: string; onselect: (date: string) => void; embedded?: boolean } = $props();
	const ledger = createLedgerSession();
	let custom = $state(false);
	let chart = $state<"categories" | "daily">("categories");
	let amount = $state("");
	let category = $state("");
	let description = $state("");
	let editingId = $state<string | null>(null);
	let draftDate = "";
	const snapshot = $derived(ledger.data?.date === selectedDate ? ledger.data : null);
	const options = $derived([...(snapshot === null ? [] : snapshot.suggestions).map((name) => ({ value: `category:${name}`, label: name })), { value: "custom", label: "Custom category…" }]);
	const currency = new Intl.NumberFormat("en-GB", { style: "currency", currency: "GBP" });
	const monthFormatter = new Intl.DateTimeFormat("en-GB", { month: "long", year: "numeric", timeZone: "UTC" });
	const monthLabel = $derived(monthFormatter.format(new Date(`${selectedDate}T00:00:00Z`)));
	const maximum = $derived(snapshot === null ? 0 : Math.max(0, ...snapshot.days.map((day) => day.amountPence)));
	const slices = $derived.by(() => {
		if (snapshot === null || snapshot.monthTotalPence === 0) return [];
		let start = 0;
		return snapshot.categories.map((item, index) => {
			const share = item.amountPence / snapshot.monthTotalPence * 100;
			const slice = { ...item, share, start, color: `var(--color-chart-${index % 7 + 1})` };
			start += share;
			return slice;
		});
	});

	$effect(() => {
		const date = selectedDate;
		untrack(() => {
			if (draftDate !== date) { draftDate = date; amount = ""; category = ""; description = ""; editingId = null; custom = false; }
			void ledger.load(date);
		});
	});

	async function save(event: SubmitEvent) {
		event.preventDefault();
		if (!amount.trim() || !category.trim() || ledger.loading) return;
		const submitted = { date: selectedDate, id: editingId, amount, category, description };
		const saved = await ledger.save(submitted.id, submitted.amount, submitted.category, submitted.description || null);
		if (saved && selectedDate === submitted.date && editingId === submitted.id && amount === submitted.amount && category === submitted.category && description === submitted.description) {
			amount = ""; category = ""; description = ""; editingId = null; custom = false;
		}
	}

	function day(offset: number) {
		const date = new Date(`${selectedDate}T00:00:00Z`);
		date.setUTCDate(date.getUTCDate() + offset);
		onselect(date.toISOString().slice(0, 10));
	}

	function edit(entry: ExpenseEntry) {
		editingId = entry.id;
		custom = false;
		amount = (entry.amountPence / 100).toFixed(2);
		category = entry.category;
		description = entry.description === null ? "" : entry.description;
	}
</script>

<section class="spending" class:embedded aria-label="Spending">
	<header>
		<div class="heading"><WalletCards size={16} /><h2>Spending</h2><span>GBP</span></div>
		<div class="date-controls"><button class="icon" type="button" aria-label="Previous day" disabled={selectedDate === "0001-01-01"} onclick={() => day(-1)}><ChevronLeft size={14} /></button><time datetime={selectedDate}>{selectedDate}</time><button class="icon" type="button" aria-label="Next day" disabled={selectedDate === "9999-12-31"} onclick={() => day(1)}><ChevronRight size={14} /></button><button type="button" disabled={selectedDate === todayDate} onclick={() => onselect(todayDate)}>Today</button></div>
	</header>
	<p class="context">Shared date with Daily Planner</p>
	{#if ledger.error !== null}<p class="error" role="alert">{ledger.error}</p>{/if}
	<div class="content">
		<div class="entry-column">
			<div class="daily-total"><span>Selected day</span><strong>{snapshot === null ? "—" : currency.format(snapshot.dayTotalPence / 100)}</strong></div>
			<form onsubmit={save} aria-label={editingId === null ? "New expense" : "Edit expense"}>
				<label>Amount (£)<input aria-label="Expense amount in GBP" type="text" inputmode="decimal" placeholder="0.00" pattern={"[0-9]+(\\.[0-9]{1,2})?"} maxlength="16" required bind:value={amount} /></label>
				<div class="category-field"><span>Category</span><Select size="compact" label="Expense category" value={custom ? "custom" : category ? `category:${category}` : ""} {options} onchange={(value: string) => { custom = value === "custom"; category = custom ? "" : value.slice(9); }} /></div>
				<div class="form-actions">
					{#if editingId !== null}<button type="button" disabled={ledger.writing} onclick={() => { editingId = null; custom = false; amount = ""; category = ""; description = ""; }}><X size={14} />Cancel</button>{/if}
					<button class="primary" aria-label={editingId === null ? "Add expense" : "Save expense"} title={editingId === null ? "Add expense" : "Save expense"} type="submit" disabled={ledger.loading || !amount.trim() || !category.trim()}>{#if editingId === null}<Plus size={14} />{:else}<Check size={14} />{/if}</button>
				</div>
				{#if custom}<label class="full-field">Custom category<input aria-label="Custom expense category" placeholder="Category name" maxlength="40" required bind:value={category} /></label>{/if}
				<label class="full-field">Note (optional)<input aria-label="Expense note" placeholder="What was this for?" maxlength="500" bind:value={description} /></label>
			</form>
			<div class="entries" aria-label={`Expenses for ${selectedDate}`}>
				{#if snapshot === null}<p>{ledger.loading ? "Loading expenses…" : "Expenses are unavailable."}</p>
				{:else if snapshot.entries.length === 0}<p>No expenses for this date.</p>
				{:else}<ul>{#each snapshot.entries as entry (entry.id)}<li><div><div class="entry-detail"><span>{entry.category}</span>{#if entry.description !== null}<p>{entry.description}</p>{/if}</div><strong>{currency.format(entry.amountPence / 100)}</strong></div><button class="icon" type="button" aria-label={`Edit ${entry.category} expense ${currency.format(entry.amountPence / 100)}`} title="Edit expense" disabled={ledger.loading} onclick={() => edit(entry)}><Pencil size={13} /></button><button class="icon" type="button" aria-label={`Delete ${entry.category} expense ${currency.format(entry.amountPence / 100)}`} title="Delete expense" disabled={ledger.loading} onclick={async () => { if (await ledger.remove(entry.id) && editingId === entry.id) { editingId = null; custom = false; amount = ""; category = ""; description = ""; } }}><Trash2 size={13} /></button></li>{/each}</ul>{/if}
			</div>
		</div>
		<div class="analytics" aria-label={`${monthLabel} spending statistics`}>
			<div class="month-heading"><div><span>Monthly overview</span><h3>{monthLabel}</h3></div><strong>{snapshot === null ? "—" : currency.format(snapshot.monthTotalPence / 100)}</strong></div>
			<div class="chart-switch" role="group" aria-label="Monthly chart view"><button type="button" aria-pressed={chart === "categories"} onclick={() => chart = "categories"}>Categories</button><button type="button" aria-pressed={chart === "daily"} onclick={() => chart = "daily"}>Daily spending</button></div>
			<div class="chart-space">
			{#if chart === "categories"}
			{#if snapshot !== null && snapshot.monthTotalPence > 0}
				<div class="category-chart">
					<svg viewBox="0 0 120 120" role="img" aria-label={`${monthLabel} spending by category`}>
						{#each slices as slice}<circle cx="60" cy="60" r="46" fill="none" stroke={slice.color} stroke-width="22" pathLength="100" stroke-dasharray={`${slice.share} ${100 - slice.share}`} stroke-dashoffset={-slice.start} transform="rotate(-90 60 60)"><title>{slice.category}: {currency.format(slice.amountPence / 100)} ({slice.share.toFixed(1)}%)</title></circle>{/each}
					</svg>
					<ul class="legend">{#each slices as slice}<li><span class="swatch" style:background={slice.color}></span><span class="category-name">{slice.category}</span><span>{slice.share.toFixed(1)}%</span><strong>{currency.format(slice.amountPence / 100)}</strong></li>{/each}</ul>
				</div>
			{:else}<div class="empty-chart"><div class="empty-ring"></div><p>{snapshot === null ? "Monthly statistics will appear here." : "No spending this month. Add your first expense."}</p></div>{/if}
			{:else}
			<div class="daily-heading"><h4>Daily spending</h4><span>{snapshot === null ? "—" : currency.format(maximum / 100)} peak</span></div>
			<div class="bar-chart" aria-label={`${monthLabel} daily spending`}>
				{#each snapshot === null ? [] : snapshot.days as day}
					<div class="day" role="img" class:selected={day.date === selectedDate} aria-label={`${day.date}: ${currency.format(day.amountPence / 100)}`} title={`${day.date}: ${currency.format(day.amountPence / 100)}`}><span class="bar-track"><span class="bar" style:height={`${maximum === 0 ? 0 : day.amountPence / maximum * 100}%`}></span></span><span class="tick">{Number(day.date.slice(8)) % 5 === 0 || day.date.endsWith("01") || day.date === selectedDate ? Number(day.date.slice(8)) : ""}</span></div>
				{/each}
			</div>
			{/if}
			</div>
		</div>
	</div>
</section>

<style>
	.spending { padding: 0.85rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header, .heading, .date-controls, .month-heading, .form-actions, .daily-heading { display: flex; align-items: center; gap: 0.5rem; }
	header, .month-heading, .daily-heading { justify-content: space-between; }
	h2, h3, h4, p { margin: 0; }
	h2 { font-size: 0.75rem; font-weight: 500; text-transform: uppercase; }
	.heading { color: var(--color-muted-foreground); }
	.heading > span { font: 0.6rem var(--font-mono); padding: 0.2rem 0.4rem; background: var(--color-muted); border-radius: var(--radius-sm); }

	.context { font-size: 0.65rem; color: var(--color-muted-foreground); margin-top: 0.45rem; }
	.content { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 1rem; margin-top: 0.65rem; }
	.entry-column { border-right: 1px solid var(--color-border); padding-right: 1rem; min-width: 0; }
	.daily-total { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 0.5rem; }
	.daily-total span, .month-heading span, label, .daily-heading span, .category-field > span { font-size: 0.68rem; color: var(--color-muted-foreground); }
	.daily-total strong, .month-heading > strong { font: 1.1rem var(--font-mono); }
	form { display: grid; grid-template-columns: minmax(4.5rem, 0.65fr) minmax(0, 1.35fr) auto; align-items: end; gap: 0.4rem; }
	.full-field { grid-column: 1 / -1; }
	form label, .category-field { display: grid; gap: 0.3rem; }
	input { box-sizing: border-box; width: 100%; min-width: 0; background: var(--color-background); border: 1px solid var(--color-border); border-radius: var(--radius-md); color: var(--color-foreground); height: 2rem; padding: 0.45rem 0.55rem; font: 0.72rem var(--font-sans); }
	input:focus-visible, button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	button { display: inline-flex; align-items: center; justify-content: center; gap: 0.3rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); padding: 0.4rem 0.6rem; background: transparent; color: var(--color-foreground); font: 0.68rem var(--font-sans); cursor: pointer; }
	button:hover:not(:disabled) { background: var(--color-muted); }
	button:disabled { opacity: 0.5; cursor: default; }
	button.primary { background: var(--color-accent); color: var(--color-accent-foreground); border-color: var(--color-accent); }
	button.primary:hover:not(:disabled) { opacity: 0.85; }
	.icon { flex: 0 0 auto; border-color: transparent; padding: 0.3rem; width: 1.75rem; height: 1.75rem; color: var(--color-muted-foreground); }
	.form-actions { justify-content: flex-end; }
	.form-actions .primary { width: 2rem; height: 2rem; padding: 0; }
	.entries { height: 7.5rem; overflow-y: auto; margin-top: 0.5rem; }
	.entries p { font-size: 0.68rem; color: var(--color-muted-foreground); line-height: 1.5; }
	ul { list-style: none; padding: 0; margin: 0; }
	.entries li { display: flex; align-items: center; gap: 0.2rem; padding: 0.3rem 0; border-bottom: 1px solid var(--color-border); }
	.entries li > div { flex: 1; min-width: 0; display: flex; justify-content: space-between; gap: 0.5rem; font-size: 0.7rem; overflow-wrap: anywhere; }
	.entry-detail { min-width: 0; }
	.entries strong { white-space: nowrap; font: 0.68rem var(--font-mono); }
	.analytics { min-width: 0; }
	h3 { font: 1rem var(--font-serif); margin-top: 0.2rem; }
	h4 { font-size: 0.68rem; font-weight: 500; }
	.category-chart, .empty-chart { display: flex; align-items: center; gap: 1rem; height: 100%; }
	.category-chart svg { width: 6.5rem; height: 6.5rem; flex: 0 0 auto; }
	.legend { flex: 1; min-width: 0; max-height: 8rem; overflow-y: auto; }
	.legend li { display: grid; grid-template-columns: 0.5rem minmax(0, 1fr) auto auto; align-items: center; gap: 0.4rem; font-size: 0.65rem; padding: 0.3rem 0; }
	.legend strong { font: 0.65rem var(--font-mono); }
	.category-name { overflow-wrap: anywhere; }
	.swatch { width: 0.45rem; height: 0.45rem; border-radius: var(--radius-sm); }
	.empty-ring { width: 6rem; height: 6rem; border: 1.1rem solid var(--color-muted); border-radius: var(--radius-full); flex: 0 0 auto; box-sizing: border-box; }
	.empty-chart p { font-size: 0.7rem; color: var(--color-muted-foreground); line-height: 1.5; }
	.bar-chart { display: flex; gap: 3px; height: 6rem; margin-top: 0.3rem; }
	.bar-chart .day { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0; border: 0; padding: 0; border-radius: var(--radius-sm); }
	.bar-track { height: 5rem; width: 100%; display: flex; align-items: flex-end; border-bottom: 1px solid var(--color-border); }
	.bar { width: 100%; min-height: 0; background: var(--color-chart-1); opacity: 0.5; border-radius: var(--radius-sm) var(--radius-sm) 0 0; }
	.bar-chart .selected .bar, .bar-chart .day:hover .bar { opacity: 1; }
	.bar-chart .selected { background: var(--color-muted); color: var(--color-accent); }
	.tick { height: 1rem; line-height: 1rem; font: 0.5rem var(--font-mono); padding-top: 0.2rem; }
	.error { margin-top: 0.5rem; font-size: 0.7rem; color: var(--color-error); }
	.chart-switch { display: flex; gap: 0.3rem; margin-top: 0.35rem; }
	.chart-switch button { padding: 0.25rem 0.5rem; }
	.chart-switch button[aria-pressed="true"] { color: var(--color-accent); background: var(--color-muted); }
	.chart-space { height: 9rem; margin-top: 0.35rem; }
	header { flex-wrap: wrap; }
	time { font: 0.65rem var(--font-mono); color: var(--color-muted-foreground); }
	.embedded .content { grid-template-columns: 1fr; }
	.embedded .entry-column { border-right: 0; padding-right: 0; }
	@media (max-width: 760px) { .content { grid-template-columns: 1fr; } .entry-column { border-right: 0; padding-right: 0; } }
</style>
