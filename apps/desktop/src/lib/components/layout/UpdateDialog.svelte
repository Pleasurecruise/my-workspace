<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onMount, tick } from "svelte";
	import type { CommandResponse, UpdateInfo, UpdateProgress } from "../../consumer";
	let { locked, onmodalchange }: { locked: boolean; onmodalchange: (open: boolean) => void } = $props();
	let updateAvailable = $state<UpdateInfo | null>(null);
	let updateProgress = $state<UpdateProgress | null>(null);
	let updateError = $state<string | null>(null);
	let updateCheckError = $state<string | null>(null);
	let updateCheckNotice = $state<string | null>(null);
	let updateChecking = $state(false);
	let installingUpdate = $state(false);
	let updateDialog = $state<HTMLDivElement | null>(null);
	let updatePercent = $derived(
		updateProgress?.status === "downloading" && updateProgress.total !== null && updateProgress.total > 0
			? Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100))
			: null,
	);
	$effect(() => {
		if (updateAvailable === null || locked) return;
		void tick().then(() => updateDialog?.focus());
	});
	let disposed = false;
	let feedbackTimer: number | null = null;
	$effect(() => { onmodalchange(updateAvailable !== null); });
	async function checkForUpdate(manual = false) {
		if (installingUpdate || disposed) return;
		if (updateChecking) {
			if (manual) {
				const message = "An update check is already running.";
				updateCheckNotice = message;
				if (feedbackTimer !== null) clearTimeout(feedbackTimer);
				feedbackTimer = window.setTimeout(() => {
					if (updateCheckNotice === message) updateCheckNotice = null;
				}, 5_000);
			}
			return;
		}
		updateChecking = true;
		updateCheckError = null;
		updateCheckNotice = null;
		const response = await invoke<CommandResponse<UpdateInfo | null>>("check_for_update");
		if (disposed) return;
		updateChecking = false;
		if (response.status === "failed") {
			updateCheckError = response.message;
			if (feedbackTimer !== null) clearTimeout(feedbackTimer);
			const message = response.message;
			feedbackTimer = window.setTimeout(() => {
				if (updateCheckError === message) updateCheckError = null;
			}, 5_000);
			return;
		}
		updateAvailable = response.data;
		if (manual && response.data === null) {
			const message = "Vesper is up to date.";
			updateCheckNotice = message;
			if (feedbackTimer !== null) clearTimeout(feedbackTimer);
			feedbackTimer = window.setTimeout(() => {
				if (updateCheckNotice === message) updateCheckNotice = null;
			}, 5_000);
		}
	}

	async function installUpdate() {
		if (updateAvailable === null || installingUpdate || disposed) return;
		installingUpdate = true;
		updateError = null;
		updateProgress = null;
		const response = await invoke<CommandResponse<string>>("install_update", {
			version: updateAvailable.version,
		});
		if (response.status === "failed") {
			installingUpdate = false;
			updateError = response.message;
		}
	}

	function keepUpdateDialogFocus(event: KeyboardEvent) {
		if (event.key !== "Tab" || updateDialog === null) return;
		const controls = updateDialog.querySelectorAll<HTMLElement>("button:not(:disabled)");
		if (controls.length === 0) return;
		const first = controls.item(0);
		const last = controls.item(controls.length - 1);
		if (document.activeElement === updateDialog) {
			event.preventDefault();
			(event.shiftKey ? last : first).focus();
		} else if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	}

	onMount(() => {
		const unlistenUpdater = listen<UpdateProgress>("updater-progress", (event) => {
			updateProgress = event.payload;
		});
		const unlistenUpdateRequest = listen("check-for-updates-requested", () => {
			void checkForUpdate(true);
		});
		void checkForUpdate();
		return () => {
			disposed = true;
			if (feedbackTimer !== null) clearTimeout(feedbackTimer);
			void unlistenUpdater.then((unlisten) => unlisten());
			void unlistenUpdateRequest.then((unlisten) => unlisten());
		};
	});
</script>

{#if updateAvailable !== null && !locked}
	<div class="update-overlay" role="presentation">
		<div bind:this={updateDialog} class="update-dialog" role="dialog" aria-modal="true" aria-labelledby="update-title" tabindex="-1" onkeydown={keepUpdateDialogFocus}>
			<p>Application update</p>
			<h1 id="update-title">Vesper {updateAvailable.version} is available</h1>
			<span>Installed version: {updateAvailable.currentVersion}</span>
			{#if updateAvailable.notes}<div class="update-notes">{updateAvailable.notes}</div>{/if}
			{#if installingUpdate}
				<div class="update-progress" class:indeterminate={updatePercent === null} role="progressbar" aria-label="Application update download" aria-valuemin="0" aria-valuemax="100" aria-valuenow={updatePercent}>
					<span style:width={updatePercent === null ? "100%" : `${updatePercent}%`}></span>
				</div>
				<small>{updateProgress?.status === "downloaded" ? "Installing and restarting…" : updatePercent === null ? "Downloading update…" : `Downloading update… ${updatePercent}%`}</small>
			{/if}
			{#if updateError}<div class="update-error" role="alert">{updateError}</div>{/if}
			<div class="update-actions">
				<button type="button" disabled={installingUpdate} onclick={() => (updateAvailable = null)}>Later</button>
				<button class="primary" type="button" disabled={installingUpdate} onclick={() => void installUpdate()}>{installingUpdate ? "Updating…" : "Download and restart"}</button>
			</div>
		</div>
	</div>
{/if}

{#if updateCheckError !== null && updateAvailable === null && !locked}
	<div class="update-check-feedback error" role="alert">
		<span>{updateCheckError}</span>
		<button type="button" aria-label="Dismiss update error" onclick={() => (updateCheckError = null)}>×</button>
	</div>
{/if}

{#if updateCheckNotice !== null && updateAvailable === null && updateCheckError === null && !locked}
	<div class="update-check-feedback" role="status">
		<span>{updateCheckNotice}</span>
		<button type="button" aria-label="Dismiss update status" onclick={() => (updateCheckNotice = null)}>×</button>
	</div>
{/if}

<style>
	.update-overlay {
		position: fixed;
		inset: 0;
		z-index: 120;
		display: grid;
		place-items: center;
		padding: 1rem;
		background: var(--color-overlay);
	}

	.update-dialog {
		display: grid;
		width: min(28rem, 100%);
		box-sizing: border-box;
		gap: 0.75rem;
		padding: 1.25rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-lg);
	}

	.update-dialog p,
	.update-dialog h1,
	.update-dialog span,
	.update-dialog small { margin: 0; }
	.update-dialog p { color: var(--color-accent); font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	.update-dialog > span,
	.update-dialog small { color: var(--color-muted-foreground); font-size: 0.72rem; }
	.update-notes { max-height: 10rem; overflow: auto; white-space: pre-wrap; font-size: 0.78rem; line-height: 1.6; }
	.update-progress { height: 0.35rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.update-progress span { display: block; height: 100%; border-radius: inherit; background: var(--color-accent); }
	.update-progress.indeterminate span { animation: update-pulse var(--duration-pulse) ease-in-out infinite alternate; }
	.update-error { color: var(--color-error); font-size: 0.75rem; }
	.update-actions { display: flex; justify-content: flex-end; gap: 0.5rem; padding-top: 0.25rem; }
	.update-actions button { height: 2rem; padding: 0 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); cursor: pointer; font-size: 0.72rem; }
	.update-actions button.primary { border-color: var(--color-accent); background: var(--color-accent); color: var(--color-accent-foreground); }
	.update-actions button:disabled { cursor: not-allowed; opacity: 0.6; }
	.update-check-feedback { position: fixed; right: 1rem; bottom: 1rem; z-index: 110; display: flex; max-width: min(32rem, calc(100vw - 2rem)); align-items: center; gap: 0.625rem; padding: 0.75rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-lg); }
	.update-check-feedback span { color: var(--color-foreground); font-size: 0.75rem; line-height: 1.4; }
	.update-check-feedback.error { border-color: var(--color-error); }
	.update-check-feedback.error span { color: var(--color-error); }
	.update-check-feedback button { padding: 0.25rem 0.5rem; border: 0; border-radius: var(--radius-sm); background: var(--color-muted); color: var(--color-foreground); cursor: pointer; font-size: 0.7rem; }
	@keyframes update-pulse { from { opacity: 0.35; } to { opacity: 1; } }

	@media (prefers-reduced-motion: reduce) { .update-progress.indeterminate span { animation: none; } }
</style>
