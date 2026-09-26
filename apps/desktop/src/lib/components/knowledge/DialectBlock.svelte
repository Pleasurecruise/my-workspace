<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount, tick } from "svelte";
	import type { Readable } from "svelte/store";
	import type { CommandResponse } from "../../consumer";
	import { mediaPlayers } from "./media";
	import { openArticleLinks } from "./links";
	import "./prose.css";

	let { source, content, context, onEdit }: { source: Readable<string>; content: HTMLElement; context: () => string; onEdit: (editing: boolean) => void } = $props();
	let host = $state<HTMLDivElement | null>(null);
	let editing = $state(false);
	let pending = $state(true);
	let preview = $state<string | null>(null);
	let error = $state("");
	let revision = $state(0);
	// ProseMirror owns this contentDOM; Svelte owns only its empty host.
	onMount(() => { host?.append(content); });
	$effect(() => {
		const text = $source;
		const request = revision;
		let active = true;
		pending = true;
		preview = null;
		error = "";
		const timer = setTimeout(() => {
			void invoke<CommandResponse<string>>("preview_knowledge", { source: text, context: context() }).then(
				(response) => {
					if (!active || request !== revision) return;
					pending = false;
					if (response.status === "failed") error = response.message;
					else preview = response.data;
				},
				(failure) => {
					if (!active || request !== revision) return;
					pending = false;
					error = `Could not compile this block: ${String(failure)}`;
				},
			);
		}, 250);
		return () => { active = false; clearTimeout(timer); };
	});
	async function editSource() {
		editing = true;
		await tick();
		onEdit(true);
	}
</script>

<div class="dialect-block">
	<div class="block-actions" contenteditable="false">
		<button type="button" onclick={editing ? () => { editing = false; onEdit(false); } : editSource}>{editing ? "Show rendered block" : "Edit Markdown"}</button>
		<button type="button" disabled={pending} onclick={() => { revision++; }}>Refresh</button>
	</div>
	<div class="block-source" hidden={!editing} bind:this={host}></div>
	{#if !editing}
		<div contenteditable="false" class="block-preview knowledge-prose" use:mediaPlayers={preview ?? ""} use:openArticleLinks={{ onError: (message) => { error = message ?? ""; }, onOpen: () => "Finish or cancel this draft before opening another article." }}>
			{#if pending}<p role="status">Compiling block…</p>
			{:else if error}<p class="block-error" role="alert">{error}</p>
			{:else if preview}{@html preview}{/if}
		</div>
	{/if}
</div>

<style>
	.dialect-block { margin: 1rem 0; border: 1px solid var(--color-border); border-radius: var(--radius-md); overflow: hidden; }
	.block-actions { display: flex; justify-content: flex-end; gap: 0.5rem; padding: 0.35rem 0.6rem; background: var(--color-muted); }
	.block-actions button { border: 0; background: transparent; color: var(--color-muted-foreground); font: inherit; font-size: 0.7rem; cursor: pointer; }
	.block-actions button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.block-source[hidden] { display: none; }
	.block-preview { white-space: normal; margin: 0; padding: 0.75rem; }
	.block-error { color: var(--color-error); }
	.block-preview :global(.content-embed) { margin-top: 0; margin-bottom: 0; }
</style>
