<script lang="ts">
	import { Check, Flame, RefreshCw } from "@lucide/svelte";
	import { createCheckInSession } from "./checkin.svelte";
	let { id, name, embedded = false }: { id: string; name: string; embedded?: boolean } = $props();
	const headingId = $props.id();
	const session = createCheckInSession(() => id);
</script>

<section class="check-in" class:embedded aria-labelledby={headingId}>
	<header><span>Daily check-in</span><button class="refresh" type="button" disabled={session.loading} onclick={() => void session.refresh()} aria-label={`Refresh ${name} check-in`}><RefreshCw size={13} /></button></header>
	<h2 id={headingId}>{name}</h2>
	{#if session.data !== null}
		{@const progress = session.data}
		<div class="stats"><span><Flame size={15} /><strong>{progress.streak}</strong> day streak</span><span>{progress.total} total</span></div>
		<div class="history" aria-label="Check-ins for the last 28 days">
			{#each progress.days as day (day.date)}<span class:done={day.completed} class:today={day.date === progress.date} title={`${day.date}: ${day.completed ? "Checked in" : "Not checked in"}`} aria-label={`${day.date}: ${day.completed ? "Checked in" : "Not checked in"}`} role="img"></span>{/each}
		</div>
		<button class="check-button" class:completed={progress.completed} type="button" disabled={session.loading} aria-pressed={progress.completed} onclick={() => void session.toggle()}>{#if progress.completed}<Check size={15} /> Checked in · Undo{:else}Check in today{/if}</button>
		<span class="date">{progress.date} · Local time</span>
	{:else if session.error === null}<p role="status">Loading check-ins…</p>{/if}
	{#if session.error !== null}<p role="alert">{session.error}</p>{/if}
</section>

<style>
	.check-in { box-sizing: border-box; display: flex; flex-direction: column; gap: 0.65rem; height: 16rem; padding: 0.85rem; overflow-y: auto; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	.check-in.embedded { height: 14rem; gap: 0.45rem; }
	header, .stats, .stats > span { display: flex; align-items: center; }
	header, .stats { justify-content: space-between; gap: 0.4rem; }
	header { color: var(--color-muted-foreground); font-size: 0.62rem; text-transform: uppercase; letter-spacing: 0.08em; }
	h2 { margin: 0; color: var(--color-foreground); font-family: var(--font-serif); font-size: 1.1rem; font-weight: 500; line-height: 1.3; overflow-wrap: anywhere; }
	.stats { color: var(--color-muted-foreground); font-size: 0.68rem; }
	.stats > span { gap: 0.3rem; }
	strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 1rem; }
	.history { display: grid; grid-template-columns: repeat(14, minmax(0, 1fr)); gap: 0.25rem; margin-top: auto; }
	.history span { aspect-ratio: 1; max-height: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-muted); }
	.history .done { background: var(--color-accent); border-color: var(--color-accent); }
	.history .today { outline: 1px solid var(--color-accent); outline-offset: 1px; }
	button { display: inline-flex; align-items: center; justify-content: center; gap: 0.4rem; cursor: pointer; }
	button:disabled { opacity: 0.5; cursor: not-allowed; }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.refresh { border: 0; background: transparent; color: var(--color-muted-foreground); padding: 0.2rem; }
	.check-button { flex-shrink: 0; min-height: 2rem; border: 1px solid var(--color-accent); border-radius: var(--radius-md); background: var(--color-accent); color: var(--color-accent-foreground); font-size: 0.72rem; }
	.check-button.completed { background: var(--color-background); color: var(--color-accent); }
	.date { text-align: center; color: var(--color-muted-foreground); font-size: 0.58rem; }
	p { margin: 0; color: var(--color-muted-foreground); font-size: 0.7rem; line-height: 1.4; }
</style>
