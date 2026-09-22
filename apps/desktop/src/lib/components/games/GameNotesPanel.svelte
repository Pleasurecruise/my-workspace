<script lang="ts">
	import { Button } from "@my-workspace/ui";
	import { RefreshCw, Star } from "@lucide/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onMount } from "svelte";
	import type { CommandResponse, Game, GameNotes, GameNotesResponse } from "../../consumer";
	import { gameNames } from "../../dashboard";
	import "./games.css";
	let { game }: { game: Game } = $props();
	let notes = $state<GameNotes | null>(null);
	let error = $state<string | null>(null);
	let notice = $state<string | null>(null);
	let loading = $state(true);
	let verificationRequired = $state(false);
	let verifying = $state(false);
	let active = false;
	let generation = 0;
	async function refresh(force = false) {
		if (force && (loading || verifying)) return;
		const current = ++generation;
		notice = null;
		loading = true;
		const result = await invoke<GameNotesResponse>("read_game_notes", { game, refresh: force });
		if (!active || current !== generation) return;
		loading = false;
		verificationRequired = result.status === "verificationRequired";
		if (result.status === "ready") { notes = result.data; error = null; }
		else if (result.status === "refreshRequired") { error = null; notice = result.message; }
		else error = result.message;
	}
	async function verify() {
		if (verifying) return;
		verifying = true;
		const result = await invoke<CommandResponse<boolean>>("verify_game", { game });
		if (!active) return;
		if (result.status === "failed") { error = result.message; verifying = false; }
	}
	onMount(() => {
		active = true;
		const subscription = listen<{ game: Game; result: GameNotesResponse }>("game-notes-updated", ({ payload }) => {
			if (!active || payload.game !== game) return;
			generation += 1;
			loading = false;
			verificationRequired = payload.result.status === "verificationRequired";
			if (payload.result.status === "ready") { notes = payload.result.data; error = null; }
			else if (payload.result.status === "refreshRequired") { error = null; notice = payload.result.message; }
			else error = payload.result.message;
		});
		const closed = listen<Game>("game-verification-closed", ({ payload }) => {
			if (!active || payload !== game) return;
			verifying = false;
		});
		const failed = listen<{ game: Game; message: string }>("game-verification-error", ({ payload }) => {
			if (active && payload.game === game) error = payload.message;
		});
		const changed = listen("game-accounts-changed", () => {
			if (!active) return;
			notes = null; error = null; notice = null; verificationRequired = false; verifying = false;
			void refresh();
		});
		void refresh();
		return () => { active = false; generation += 1; for (const pending of [closed, failed, subscription, changed]) void pending.then((unlisten) => unlisten()); };
	});
</script>

<section class="game-panel compact" aria-label={`${gameNames[game]} daily notes`}>
	<header><h3>Daily status</h3><button type="button" class="refresh" class:spinning={loading} disabled={loading || verifying} onclick={() => refresh(true)} aria-label="Refresh daily status" title="Refresh daily status"><RefreshCw size={13} /></button></header>
	{#if loading}<p class="muted">Loading daily notes…</p>{/if}
	{#if error !== null}<p class="error" role="alert">{error}</p>{/if}
	{#if notice !== null}<p class="muted" role="status">{notice}</p>{/if}
	{#if verificationRequired}<Button variant="outline" size="sm" disabled={verifying} onclick={verify}>{verifying ? "Complete verification in the opened window…" : game === "zzz" ? "Open official game record" : "Verify now"}</Button>{/if}
	{#if notes !== null}
		<p class="muted">{notes.account.name} · {notes.account.uid}</p>
		<div class="metrics">
			{#each notes.meters as meter (meter.label)}
				<div><dl><dt>{meter.label}</dt><dd>{meter.current} / {meter.max}</dd></dl>
				{#if meter.fullAt !== null && meter.current < meter.max}<p class="muted">Full at {new Date(meter.fullAt * 1000).toLocaleString()}</p>{/if}</div>
			{/each}
			{#each notes.tasks as task (task.label)}<dl><dt>{task.label}</dt><dd>{#if task.progress !== null && task.progress.total > 0 && task.progress.total <= 5}<span class="task-stars" role="img" aria-label={`${task.label}: ${task.progress.current} of ${task.progress.total}`} title={`${task.label}: ${task.value}`}>{#each Array.from({ length: task.progress.total }, (_, index) => index) as index (index)}<Star size={15} fill={index < task.progress.current ? "currentColor" : "none"} aria-hidden="true" />{/each}</span>{:else}{task.value}{/if}</dd></dl>{/each}
		</div>
		<p class="muted">Updated {new Date(notes.sampledAt * 1000).toLocaleTimeString()}</p>
	{/if}
</section>

<style>
	.refresh { display: grid; place-items: center; width: 1.6rem; height: 1.6rem; padding: 0; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.refresh:hover { color: var(--color-foreground); background: var(--color-muted); }
	.refresh:disabled { cursor: default; opacity: 0.5; }
	.refresh:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.spinning :global(svg) { animation: spin var(--duration-spinner) linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }
	@media (prefers-reduced-motion: reduce) { .spinning :global(svg) { animation: none; } }
	.task-stars { display: inline-flex; align-items: center; gap: 0.3rem; color: var(--color-accent); }
	h3 { margin: 0; font-size: 0.75rem; font-weight: 500; color: var(--color-muted-foreground); }
	.compact .metrics { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.5rem 1rem; margin: 0.75rem 0; }
	.compact .metrics dl { grid-template-columns: 1fr; gap: 0.2rem; margin: 0; padding: 0; }
	.compact .metrics p { margin: 0.15rem 0 0; font-size: 0.6rem; }
	dl { padding: 0.35rem 0; }
	dd { font-family: var(--font-mono); font-size: 0.95rem; }
</style>
