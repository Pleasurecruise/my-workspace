<script lang="ts">
	import { CalendarDays, ChevronLeft, ChevronRight } from "@lucide/svelte";

	let { todayDate, selectedDate, onselect }: { todayDate: string; selectedDate: string; onselect: (date: string) => void | Promise<void> } = $props();
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
</script>

	<section class="calendar-panel" aria-label="Calendar">
	<header><div><CalendarDays size={15} /><h2>Calendar</h2></div><button type="button" disabled={selectedDate === todayDate && monthOffset === 0} onclick={() => { monthOffset = 0; void onselect(todayDate); }}>Today</button></header>
	<div class="month-heading"><button type="button" disabled={!calendar.canGoBack} onclick={() => (monthOffset -= 1)} aria-label="Previous month"><ChevronLeft size={13} /></button><strong>{calendar.label}</strong><button type="button" disabled={!calendar.canGoForward} onclick={() => (monthOffset += 1)} aria-label="Next month"><ChevronRight size={13} /></button></div>
	<div class="month-calendar" aria-label={`${calendar.label} calendar`}>
		{#each weekdays as weekday}<span class="weekday">{weekday}</span>{/each}
		{#each Array(calendar.leadingDays) as _}<span class="calendar-spacer"></span>{/each}
		{#each calendar.days as day}<button type="button" class:today={day.date === todayDate} class:selected={day.date === selectedDate} onclick={() => { monthOffset = 0; void onselect(day.date); }} aria-label={`Select ${day.date}`} aria-pressed={day.date === selectedDate}>{day.number}</button>{/each}
	</div>
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
	.month-calendar button { width: 100%; font-family: var(--font-mono); font-size: 0.6rem; }
	.month-calendar button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-foreground); }
	.month-calendar button.today { color: var(--color-accent); box-shadow: inset 0 0 0 1px var(--color-accent); }
	.month-calendar button.selected { background: var(--color-accent); color: var(--color-accent-foreground); }
	button:disabled { cursor: not-allowed; opacity: 0.45; }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
</style>
