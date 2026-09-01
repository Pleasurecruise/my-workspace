<script lang="ts">
	import { CalendarDays, ChevronLeft, ChevronRight } from "@lucide/svelte";

	let { todayDate, selectedDate, loading, onselect }: { todayDate: string; selectedDate: string; loading: boolean; onselect: (date: string) => Promise<void> } = $props();
	const monthFormatter = new Intl.DateTimeFormat("en-US", { month: "long", timeZone: "UTC", year: "numeric" });
	const weekdays = ["M", "T", "W", "T", "F", "S", "S"];
	let monthOffset = $state(0);
	let calendar = $derived.by(() => {
		const selected = new Date(`${selectedDate}T00:00:00Z`);
		const first = new Date(Date.UTC(selected.getUTCFullYear(), selected.getUTCMonth() + monthOffset, 1));
		const dayCount = new Date(Date.UTC(first.getUTCFullYear(), first.getUTCMonth() + 1, 0)).getUTCDate();
		return { label: monthFormatter.format(first), leadingDays: (first.getUTCDay() + 6) % 7, days: Array.from({ length: dayCount }, (_, index) => { const day = new Date(Date.UTC(first.getUTCFullYear(), first.getUTCMonth(), index + 1)); return { date: day.toISOString().slice(0, 10), number: index + 1 }; }) };
	});
</script>

	<section class="calendar-panel" aria-label="Calendar">
	<header><div><CalendarDays size={15} /><h2>Calendar</h2></div><span>{selectedDate}</span></header>
	<div class="month-heading"><button type="button" onclick={() => (monthOffset -= 1)} aria-label="Previous month"><ChevronLeft size={13} /></button><strong>{calendar.label}</strong><button type="button" onclick={() => (monthOffset += 1)} aria-label="Next month"><ChevronRight size={13} /></button></div>
	<div class="month-calendar" aria-label={`${calendar.label} calendar`}>
		{#each weekdays as weekday}<span class="weekday">{weekday}</span>{/each}
		{#each Array(calendar.leadingDays) as _}<span class="calendar-spacer"></span>{/each}
		{#each calendar.days as day}<button type="button" class:today={day.date === todayDate} class:selected={day.date === selectedDate} disabled={loading} onclick={() => { monthOffset = 0; void onselect(day.date); }} aria-label={`Select ${day.date}`} aria-pressed={day.date === selectedDate}>{day.number}</button>{/each}
	</div>
</section>

<style>
	.calendar-panel { width: 100%; min-width: 0; box-sizing: border-box; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	header,
	header div { display: flex; align-items: center; }
	header { justify-content: space-between; gap: 0.5rem; margin-bottom: 0.9rem; }
	header div { gap: 0.4rem; }
	h2 { margin: 0; color: var(--color-muted-foreground); font-size: 0.72rem; font-weight: 500; text-transform: uppercase; }
	header > span { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.58rem; }
	.month-heading { display: grid; grid-template-columns: 1.5rem 1fr 1.5rem; align-items: center; margin-bottom: 0.45rem; }
	.month-heading strong { font-size: 0.68rem; font-weight: 550; text-align: center; }
	button { display: inline-flex; height: 1.55rem; align-items: center; justify-content: center; padding: 0; border: 1px solid transparent; border-radius: var(--radius-md); background: var(--color-background); color: var(--color-muted-foreground); cursor: pointer; }
	.month-heading button { width: 1.5rem; }
	.month-calendar { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 0.15rem; }
	.month-calendar .weekday { padding: 0.15rem 0; color: var(--color-muted-foreground); font-size: 0.5rem; text-align: center; }
	.month-calendar button { width: 100%; font-family: var(--font-mono); font-size: 0.6rem; }
	.month-calendar button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-foreground); }
	.month-calendar button.today { color: var(--color-accent); box-shadow: inset 0 0 0 1px var(--color-accent); }
	.month-calendar button.selected { background: var(--color-accent); color: var(--color-accent-foreground); }
	button:disabled { cursor: not-allowed; opacity: 0.45; }
</style>
