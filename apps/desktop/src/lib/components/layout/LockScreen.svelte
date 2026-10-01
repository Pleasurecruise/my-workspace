<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import type { CommandResponse } from "../../contracts/command";

	let { onunlock }: { onunlock: () => void } = $props();

	let password = $state("");
	let error = $state<string | null>(null);
	let unlocking = $state(false);
	let input = $state<HTMLInputElement | null>(null);

	onMount(() => input?.focus());

	async function unlock(event: SubmitEvent) {
		event.preventDefault();
		if (unlocking || password === "") return;
		unlocking = true;
		error = null;
		const response = await invoke<CommandResponse<string>>("unlock_app", { password });
		unlocking = false;
		password = "";
		if (response.status === "failed") {
			error = response.message;
			input?.focus();
			return;
		}
		onunlock();
	}
</script>

<div class="lock-screen" role="dialog" aria-modal="true" aria-labelledby="lock-title">
	<div class="lock-card">
		<h1 id="lock-title">Locked</h1>
		<form onsubmit={unlock}>
			<label for="unlock-password">Password</label>
			<input bind:this={input} id="unlock-password" type="password" bind:value={password} autocomplete="current-password" placeholder="Enter password" />
			{#if error}<p class="unlock-error" role="alert">{error}</p>{/if}
			<button type="submit" disabled={unlocking || password === ""}>{unlocking ? "Unlocking…" : "Unlock"}</button>
		</form>
	</div>
</div>

<style>
	.lock-screen {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: grid;
		place-items: center;
		padding: 1.5rem;
		background: color-mix(in srgb, var(--color-background) 96%, var(--color-muted));
	}

	.lock-card {
		display: grid;
		width: min(100%, 18rem);
		justify-items: center;
		gap: 0.9rem;
		box-sizing: border-box;
		padding: 1rem;
		text-align: center;
	}

	.lock-card h1 { margin: 0 0 0.35rem; }
	.lock-card form { display: grid; width: 100%; gap: 0.55rem; text-align: left; }
	.lock-card label { color: var(--color-muted-foreground); font-size: 0.65rem; }
	.lock-card input { min-width: 0; height: 2rem; box-sizing: border-box; padding: 0 0.625rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); outline: none; background: var(--color-background); color: var(--color-foreground); font-size: 0.75rem; }
	.lock-card input:focus { border-color: var(--color-accent); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 14%, transparent); }
	.lock-card button { height: 1.75rem; margin-top: 0.15rem; padding: 0 0.625rem; border: 1px solid var(--color-accent); border-radius: var(--radius-md); background: var(--color-accent); color: var(--color-accent-foreground); cursor: pointer; font-size: 0.68rem; font-weight: 400; }
	.lock-card button:disabled { cursor: wait; opacity: 0.55; }
	.unlock-error { margin: 0.2rem 0 0; color: var(--color-error); font-size: 0.68rem; }
</style>
