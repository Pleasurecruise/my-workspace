<script lang="ts">
	import "./settings.css";
	import { invoke } from "@tauri-apps/api/core";
	import { onDestroy, tick } from "svelte";
	import { LoaderCircle, Music2, QrCode, X } from "@lucide/svelte";
	import { Alert, AlertDescription, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Label } from "@my-workspace/ui";
	import type { CommandResponse, QqLoginStatus, QqQr } from "../../consumer";
	import ConfigurationBadge from "./ConfigurationBadge.svelte";
	let { connected, onconnected }: { connected: boolean; onconnected: () => Promise<void> } = $props();
	let starting = $state(false);
	let cancelling = $state(false);
	let error = $state<string | null>(null);
	let qqQr = $state<QqQr | null>(null);
	let qqDialog = $state<HTMLDivElement | null>(null);
	let qqStatus = $state<"waiting" | "scanned" | "expired">("waiting");
	let generation = 0;
	let disposed = false;
	let timer: ReturnType<typeof setTimeout> | null = null;

	async function begin() {
		if (starting || cancelling || disposed || qqQr !== null) return;
		const version = ++generation;
		starting = true;
		error = null;
		const response = await invoke<CommandResponse<QqQr>>("begin_qq_music_login");
		if (disposed || version !== generation) return;
		starting = false;
		if (response.status === "failed") { error = response.message; return; }
		qqQr = response.data;
		qqStatus = "waiting";
		await tick();
		if (disposed || version !== generation) return;
		qqDialog?.focus();
		timer = setTimeout(() => void poll(version), 1_500);
	}

	async function poll(version: number) {
		if (disposed || version !== generation || qqQr === null) return;
		const response = await invoke<CommandResponse<QqLoginStatus>>("poll_qq_music_login");
		if (disposed || version !== generation) return;
		if (response.status === "failed") { error = response.message; qqQr = null; return; }
		if (response.data.status === "complete") {
			qqQr = null;
			await onconnected();
			return;
		}
		qqStatus = response.data.status;
		if (qqStatus !== "expired") timer = setTimeout(() => void poll(version), 1_500);
	}

	async function close() {
		++generation;
		if (timer !== null) clearTimeout(timer);
		qqQr = null;
		starting = false;
		cancelling = true;
		const response = await invoke<CommandResponse<null>>("cancel_qq_music_login");
		if (disposed) return;
		cancelling = false;
		if (response.status === "failed") error = response.message;
	}

	onDestroy(() => {
		disposed = true;
		++generation;
		if (timer !== null) clearTimeout(timer);
		if (starting || qqQr !== null) void invoke<CommandResponse<null>>("cancel_qq_music_login");
	});
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape" && qqQr !== null) void close(); }} />

{#if error !== null}<Alert variant="error"><AlertDescription>{error}</AlertDescription></Alert>{/if}
<Card>
	<CardHeader class="settings-card-header settings-card-header-status">
		<span class="settings-icon"><Music2 size={16} /></span>
		<div><CardTitle class="settings-card-title">QQ Music</CardTitle><CardDescription class="settings-card-description">Play the personalized Daily 30 recommendation from your QQ Music account.</CardDescription></div>
		{#if connected}<ConfigurationBadge />{/if}
	</CardHeader>
	<CardContent class="settings-card-content">
		<div class="settings-row">
			<div><Label>QR authorization</Label><p>Scan with the QQ mobile app. Vesper stores the resulting renewable session automatically.</p></div>
			<Button class="settings-save-button" size="sm" type="button" disabled={starting || cancelling} onclick={begin}>{starting ? "Creating QR code…" : connected ? "Reconnect" : "Connect"}</Button>
		</div>
	</CardContent>
</Card>

{#if qqQr !== null}
	<div class="qq-login-backdrop" role="presentation" onclick={(event) => { if (event.currentTarget === event.target) close(); }}>
		<div bind:this={qqDialog} class="qq-login" role="dialog" aria-modal="true" aria-labelledby="qq-login-title" tabindex="-1">
			<Button class="qq-login-close" variant="ghost" size="icon" type="button" onclick={close} aria-label="Close QQ Music login"><X size={16} /></Button>
			<div class="qq-login-icon"><QrCode size={18} /></div>
			<h2 id="qq-login-title">Connect QQ Music</h2>
			<p>Open QQ on your phone and scan this code to authorize Vesper.</p>
			<div class:expired={qqStatus === "expired"} class="qq-code"><img src={qqQr.image} alt="QQ Music login QR code" /></div>
			<div class="qq-login-status">
				{#if qqStatus === "expired"}<span>QR code expired. Close and connect again.</span>{:else}<LoaderCircle class="qq-login-spinner" size={14} /><span>{qqStatus === "scanned" ? "Scanned — confirm on your phone" : "Waiting for scan"}</span>{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.qq-login-backdrop { position: fixed; inset: 0; z-index: 100; display: grid; place-items: center; box-sizing: border-box; padding: 1rem; background: var(--color-overlay); backdrop-filter: blur(10px); }
	.qq-login { position: relative; display: grid; width: min(100%, 22rem); justify-items: center; box-sizing: border-box; padding: 1.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-lg); text-align: center; }
	:global(.qq-login-close) { position: absolute; top: 0.75rem; right: 0.75rem; }
	.qq-login-icon { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-accent); }
	.qq-login h2 { margin: 0.75rem 0 0; }
	.qq-login > p { max-width: 17rem; margin: 0.4rem 0 1rem; color: var(--color-muted-foreground); font-size: 0.75rem; line-height: 1.5; }
	.qq-code { display: grid; width: 12rem; height: 12rem; place-items: center; padding: 0.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); transition: opacity var(--duration-base); }
	.qq-code.expired { opacity: 0.28; }
	.qq-code img { display: block; width: 100%; height: 100%; image-rendering: pixelated; }
	.qq-login-status { display: flex; min-height: 1.25rem; align-items: center; gap: 0.4rem; margin-top: 1rem; color: var(--color-muted-foreground); font-size: 0.7rem; }
	:global(.qq-login-spinner) { animation: qq-login-spin var(--duration-slow) linear infinite; }
	@keyframes qq-login-spin { to { rotate: 360deg; } }
	@media (prefers-reduced-motion: reduce) { :global(.qq-login-spinner) { animation: none; } }
</style>
