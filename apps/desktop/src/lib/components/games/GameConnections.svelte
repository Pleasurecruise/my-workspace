<script lang="ts">
	import ConfigurationBadge from "../settings/ConfigurationBadge.svelte";
	import { X } from "@lucide/svelte";
	import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Input, Label, Select } from "@my-workspace/ui";
	import { invoke } from "@tauri-apps/api/core";
	import { onDestroy, onMount, tick, untrack } from "svelte";
	import type { CommandResponse, Game, GameConnections, GameLoginProgress, GameLoginQr, SteamSettings } from "../../consumer";
	import { gameNames } from "../../dashboard";
	let { reconnectMihoyo }: { reconnectMihoyo: boolean } = $props();
	$effect(() => { if (reconnectMihoyo) untrack(() => { void connect("mihoyo"); }); });
	let connections = $state<GameConnections>({ providers: [], mihoyo: [], bindings: [] });
	let changing = $state(false);
	let revision = 0;
	let connectionError = $state<string | null>(null);
	let provider = $state<"mihoyo" | "skland" | null>(null);
	let qr = $state<GameLoginQr | null>(null);
	let loginError = $state<string | null>(null);
	let progress = $state<GameLoginProgress>("waiting");
	let creating = $state(false);
	let generation = 0;
	let timer: ReturnType<typeof setTimeout> | null = null;
	let dialog = $state<HTMLDialogElement | null>(null);
	let apiKey = $state("");
	let steamId = $state("");
	let savedKey = $state("");
	let savedId = $state("");
	let steamLoading = $state(true);
	let keyEdited = false;
	let idEdited = false;
	let steamChanged = $derived(apiKey.trim() !== savedKey || steamId.trim() !== savedId);
	let steamSaving = $state(false);
	let steamError = $state<string | null>(null);
	let active = true;
	const providers: Array<{ id: "mihoyo" | "skland"; name: string; description: string }> = [
		{ id: "mihoyo", name: "miHoYo", description: "Genshin Impact, Honkai: Star Rail and Zenless Zone Zero · Official CN servers" },
		{ id: "skland", name: "Skland", description: "Arknights and Arknights: Endfield · Official CN servers" },
	];
	async function refresh() {
		const current = ++revision;
		const result = await invoke<CommandResponse<GameConnections>>("read_game_connections");
		if (!active || current !== revision) return;
		if (result.status === "ready") connections = result.data;
		else connectionError = result.message;
	}
	onMount(() => {
		void refresh();
		void invoke<CommandResponse<SteamSettings | null>>("read_steam_settings").then((result) => {
			if (!active) return;
			steamLoading = false;
			if (result.status === "failed") { steamError = result.message; return; }
			if (result.data !== null) {
				savedKey = result.data.apiKey; savedId = result.data.steamId;
				if (!keyEdited) apiKey = savedKey;
				if (!idEdited) steamId = savedId;
			}
		});
	});
	async function select(game: Game, id: string) {
		if (changing) return;
		changing = true; connectionError = null; revision += 1;
		const result = await invoke<CommandResponse<null>>("select_game_account", { game, id: id === "" ? null : id });
		if (!active) return;
		if (result.status === "failed") connectionError = result.message;
		await refresh();
		changing = false;
	}
	async function remove(id: string) {
		if (changing) return;
		changing = true; connectionError = null; revision += 1;
		const result = await invoke<CommandResponse<null>>("remove_game_account", { id });
		if (!active) return;
		if (result.status === "failed") connectionError = result.message;
		await refresh();
		changing = false;
	}
	function closeLogin() {
		generation += 1;
		if (timer !== null) { clearTimeout(timer); timer = null; }
		if (provider !== null && qr !== null) void invoke("cancel_game_login", { provider, id: qr.id });
		dialog?.close(); provider = null; qr = null; creating = false;
	}
	onDestroy(() => { active = false; closeLogin(); });
	async function connect(selected: "mihoyo" | "skland") {
		closeLogin();
		const current = generation;
		provider = selected; creating = true; loginError = null; progress = "waiting";
		await tick();
		if (current !== generation) return;
		dialog?.showModal();
		const result = await invoke<CommandResponse<GameLoginQr>>("begin_game_login", { provider: selected });
		if (current !== generation) {
			if (result.status === "ready") void invoke("cancel_game_login", { provider: selected, id: result.data.id });
			return;
		}
		creating = false;
		if (result.status === "failed") { loginError = result.message; return; }
		qr = result.data;
		void poll(selected, result.data, current);
	}
	async function poll(selected: "mihoyo" | "skland", code: GameLoginQr, current: number) {
		const result = await invoke<CommandResponse<GameLoginProgress>>("poll_game_login", { provider: selected, id: code.id });
		if (current !== generation) return;
		if (result.status === "failed") { loginError = result.message; return; }
		progress = result.data;
		if (progress === "complete") {
			closeLogin();
			await refresh();
			return;
		}
		if (progress === "expired") return;
		timer = setTimeout(() => { timer = null; void poll(selected, code, current); }, 2000);
	}
	async function saveSteam() {
		if (steamLoading || steamSaving || !steamChanged || !apiKey.trim() || !/^\d{17}$/.test(steamId.trim())) return;
		const submittedKey = apiKey.trim();
		const submittedId = steamId.trim();
		steamSaving = true; steamError = null;
		const result = await invoke<CommandResponse<null>>("save_steam_connection", { apiKey: submittedKey, steamId: submittedId });
		if (!active) return;
		steamSaving = false;
		if (result.status === "failed") { steamError = result.message; return; }
		void refresh();
		savedKey = submittedKey;
		savedId = submittedId;
	}
</script>

<Card>
	<CardHeader><CardTitle>Games</CardTitle><CardDescription>Connect an account for daily notes and permanent pull archives.</CardDescription></CardHeader>
	<CardContent>
		{#if connectionError !== null}<p class="error" role="alert">{connectionError}</p>{/if}
		{#each providers as item (item.id)}
			<div class="connection"><div><strong>{item.name}</strong><p>{item.description}</p>{#if connections.providers.includes(item.id)}<ConfigurationBadge label={item.id === "mihoyo" ? `${connections.mihoyo.length} accounts` : "Configured"} />{/if}</div><Button size="sm" disabled={changing} onclick={() => connect(item.id)}>{item.id === "mihoyo" && connections.mihoyo.length > 0 ? "Add account" : connections.providers.includes(item.id) ? "Reconnect" : "Scan to connect"}</Button></div>
			{#if item.id === "mihoyo" && connections.mihoyo.length > 0}
				<div class="accounts">
					{#each connections.mihoyo as id (id)}
						<div class="account"><span>miHoYo · {id}</span><Button variant="ghost" size="sm" disabled={changing} onclick={() => remove(id)} aria-label={`Remove miHoYo account ${id}`}>Remove</Button></div>
					{/each}
					<p>Scan again to renew an existing account. Removing a login keeps its pull archive.</p>
					<div class="bindings">
						{#each connections.bindings as binding (binding.game)}
							<div class="binding"><span>{gameNames[binding.game]}</span><Select label={`${gameNames[binding.game]} account`} value={binding.accountId === null ? "" : binding.accountId} options={[{ value: "", label: "Choose account" }, ...connections.mihoyo.map((id) => ({ value: id, label: `miHoYo · ${id}` }))]} disabled={changing} onchange={(id: string) => select(binding.game, id)} /></div>
						{/each}
					</div>
					<p>Daily Notes and Pull Analysis use the selected account for each game.</p>
				</div>
			{/if}
		{/each}
		<form onsubmit={(event) => { event.preventDefault(); void saveSteam(); }}>
			<div class="connection"><div><strong>Steam</strong><p>Use your personal Steam Web API key and SteamID64.</p>{#if connections.providers.includes("steam")}<ConfigurationBadge />{/if}</div><Button size="sm" type="submit" disabled={steamLoading || steamSaving || !steamChanged || !apiKey.trim() || !/^\d{17}$/.test(steamId.trim())}>{steamSaving ? "Saving…" : "Save"}</Button></div>
			<div class="fields"><div><Label for="games-steam-id">SteamID64</Label><Input id="games-steam-id" oninput={() => { idEdited = true; }} autocomplete="off" bind:value={steamId} /></div><div><Label for="games-steam-key">API key</Label><Input id="games-steam-key" oninput={() => { keyEdited = true; }} type="password" autocomplete="off" bind:value={apiKey} /></div></div>
			{#if steamLoading}<p role="status">Loading saved configuration…</p>{/if}
			{#if steamError !== null}<p class="error" role="alert">{steamError}</p>{/if}
		</form>
	</CardContent>
</Card>

<dialog bind:this={dialog} oncancel={(event) => { event.preventDefault(); closeLogin(); }} aria-labelledby="games-login-title">
	<div class="dialog-content"><div class="dialog-header"><h2 id="games-login-title">Connect {provider === "mihoyo" ? "miHoYo" : "Skland"}</h2><Button variant="ghost" size="icon" onclick={closeLogin} aria-label="Close game login"><X size={16} /></Button></div>
		<p>Scan with {provider === "mihoyo" ? "the Mihoyo mobile app" : "the Skland mobile app"} and approve the login on your phone.</p>
		{#if creating}<p role="status">Creating QR code…</p>{/if}
		{#if qr !== null}<img class:expired={progress === "expired"} src={qr.image} alt="Game account login QR code" width="256" height="256" /><p role="status">{progress === "expired" ? "QR code expired." : progress === "scanned" ? "Confirm login on your phone." : "Waiting for scan…"}</p>{/if}
		{#if loginError !== null}<p class="error" role="alert">{loginError}</p>{/if}
		{#if provider !== null && (progress === "expired" || loginError !== null)}<Button onclick={() => { if (provider !== null) void connect(provider); }}>New QR code</Button>{/if}
	</div>
</dialog>

<style>
	.connection { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1rem; }
	strong { font-size: 0.85rem; }
	p { margin: 0.3rem 0; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.error { color: var(--color-error); }
	.fields { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1rem; }
	.fields > div { display: grid; gap: 0.4rem; }
	.accounts { margin-bottom: 1.5rem; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
	.account { display: flex; align-items: center; justify-content: space-between; gap: 1rem; font-size: 0.8rem; }
	.bindings { display: grid; gap: 0.75rem; margin: 1rem 0; }
	.binding { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 1rem; align-items: center; font-size: 0.8rem; }
	@media (max-width: 600px) { .binding, .fields { grid-template-columns: 1fr; } }
	form { border-top: 1px solid var(--color-border); padding-top: 1rem; }
	dialog { position: fixed; inset: 0; width: 100%; max-width: none; height: 100%; max-height: none; margin: 0; padding: 1.5rem; border: 0; box-sizing: border-box; background: var(--color-overlay); color: var(--color-foreground); backdrop-filter: blur(8px); }
	dialog[open] { display: grid; place-items: center; }
	dialog::backdrop { background: transparent; }
	.dialog-content { width: min(100%, 25rem); max-height: 100%; overflow-y: auto; box-sizing: border-box; padding: 1.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-lg); text-align: center; }
	.dialog-header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1rem; }
	h2 { margin: 0; }
	img { display: block; width: 16rem; max-width: 100%; height: auto; margin: 1rem auto; border-radius: var(--radius-md); }
	.expired { opacity: 0.3; }
</style>
