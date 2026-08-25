<script lang="ts">
	import { BellRing, CircleCheck, Cloud, Eye, EyeOff, KeyRound, Lock } from "@lucide/svelte";
	import {
		Alert,
		AlertDescription,
		Button,
		Card,
		CardContent,
		CardDescription,
		CardFooter,
		CardHeader,
		CardTitle,
		Input,
		Label,
	} from "@my-workspace/ui";
	import type {
		ApiConfiguration,
		CommandResponse,
		ConfigurationStatus,
		NtfyConfig,
		R2Configuration,
		UgosConfiguration,
	} from "../consumer";

	let {
		configuration,
		error,
		onsaveugos,
		onsaver2,
		onsaveapi,
		onsaventfy,
		onsaveapplock,
		onremoveapplock,
	}: {
		configuration: ConfigurationStatus | null;
		error: string | null;
		onsaveugos: (input: UgosConfiguration) => Promise<CommandResponse<string>>;
		onsaver2: (input: R2Configuration) => Promise<CommandResponse<string>>;
		onsaveapi: (input: ApiConfiguration) => Promise<CommandResponse<string>>;
		onsaventfy: (configuration: NtfyConfig) => Promise<CommandResponse<string>>;
		onsaveapplock: (password: string) => Promise<CommandResponse<string>>;
		onremoveapplock: () => Promise<CommandResponse<string>>;
	} = $props();

	let username = $state("");
	let password = $state("");
	let accessKeyId = $state("");
	let secretAccessKey = $state("");
	let memosApiKey = $state("");
	let momentApiKey = $state("");
	let knowledgeApiKey = $state("");
	let ntfyToken = $state("");
	let appLockPassword = $state("");
	let saving = $state<"app-lock" | "ugos" | "r2" | "memos" | "moment" | "knowledge" | "ntfy" | null>(null);
	let formError = $state<string | null>(null);
	let passwordVisible = $state(false);
	let secretVisible = $state(false);
	let memosKeyVisible = $state(false);
	let momentKeyVisible = $state(false);
	let knowledgeKeyVisible = $state(false);
	let ntfyTokenVisible = $state(false);
	let appLockPasswordVisible = $state(false);

	$effect(() => {
		if (configuration !== null && configuration.ugos.status === "ready") {
			username = configuration.ugos.data.username;
			password = configuration.ugos.data.password;
		}
		if (configuration !== null && configuration.r2.status === "ready") {
			accessKeyId = configuration.r2.data.accessKeyId;
			secretAccessKey = configuration.r2.data.secretAccessKey;
		}
		if (configuration !== null && configuration.api.memos.status === "ready") memosApiKey = configuration.api.memos.data;
		if (configuration !== null && configuration.api.moment.status === "ready") momentApiKey = configuration.api.moment.data;
		if (configuration !== null && configuration.api.knowledge.status === "ready") knowledgeApiKey = configuration.api.knowledge.data;
		if (configuration !== null && configuration.ntfy.status === "ready") {
			ntfyToken = configuration.ntfy.data.token;
		}
		if (configuration !== null && configuration.appLock.status === "ready") appLockPassword = configuration.appLock.data;
	});

	async function saveUgos(event: SubmitEvent) {
		event.preventDefault();
		saving = "ugos";
		formError = null;
		const response = await onsaveugos({ username, password });
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
	}

	async function saveR2(event: SubmitEvent) {
		event.preventDefault();
		saving = "r2";
		formError = null;
		const response = await onsaver2({ accessKeyId, secretAccessKey });
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
	}

	async function saveApi(service: "memos" | "moment" | "knowledge") {
		let apiKey = knowledgeApiKey;
		if (service === "memos") apiKey = memosApiKey;
		if (service === "moment") apiKey = momentApiKey;
		if (apiKey.trim() === "") return;
		saving = service;
		formError = null;
		const response = await onsaveapi({ service, apiKey });
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
	}

	async function saveAppLock(event: SubmitEvent) {
		event.preventDefault();
		formError = null;
		saving = "app-lock";
		const response = await onsaveapplock(appLockPassword);
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		appLockPasswordVisible = false;
	}

	async function saveNtfy(event: SubmitEvent) {
		event.preventDefault();
		saving = "ntfy";
		formError = null;
		const response = await onsaventfy({
			token: ntfyToken,
			development: false,
		});
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
	}

	async function removeAppLock() {
		saving = "app-lock";
		formError = null;
		const response = await onremoveapplock();
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		appLockPassword = "";
		appLockPasswordVisible = false;
	}
</script>

<section class="settings" aria-label="Vesper configuration">
	<header>
		<div><p>System credentials</p><h1>Settings</h1></div>
	</header>

	{#if formError !== null}
		<Alert class="settings-alert" variant="error"><AlertDescription class="settings-alert-copy">{formError}</AlertDescription></Alert>
	{:else if error !== null}
		<Alert class="settings-alert" variant="error"><AlertDescription class="settings-alert-copy">{error}</AlertDescription></Alert>
	{/if}

	<div class="sections">
	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon"><BellRing size={16} /></span>
			<div><CardTitle class="settings-card-title">Notifications</CardTitle><CardDescription class="settings-card-description">Subscribe to notifications through ntfy.</CardDescription></div>
			{#if configuration?.ntfyDev}<span class="configured"><CircleCheck size={14} /> Environment</span>{:else if configuration?.ntfy.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
		</CardHeader>
		<form onsubmit={saveNtfy}>
			<CardContent class="settings-card-content">
				<div class="setting-row">
					<div><Label for="ntfy-token">Token</Label><p>Use the access token for the Vesper ntfy subscription.</p></div>
					<div class="secret-field">
						<Input id="ntfy-token" class="settings-input settings-secret-input" type={ntfyTokenVisible ? "text" : "password"} bind:value={ntfyToken} autocomplete="off" autocapitalize="none" spellcheck="false" required />
						<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (ntfyTokenVisible = !ntfyTokenVisible)} aria-label={ntfyTokenVisible ? "Hide ntfy token" : "Show ntfy token"}>{#if ntfyTokenVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
					</div>
				</div>
			</CardContent>
			<CardFooter class="settings-card-footer"><span class="settings-footer-copy">Vesper uses one fixed authenticated ntfy SSE subscription.</span><Button class="settings-save-button" size="sm" type="submit" disabled={saving !== null || ntfyToken.trim() === ""}>{saving === "ntfy" ? "Saving…" : "Save"}</Button></CardFooter>
		</form>
	</Card>

	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon"><Lock size={16} /></span>
			<div><CardTitle class="settings-card-title">App Lock</CardTitle><CardDescription class="settings-card-description">Hide Vesper behind a local password while it remains open.</CardDescription></div>
			{#if configuration?.appLockDev}<span class="configured"><CircleCheck size={14} /> Environment</span>{:else if configuration?.appLock.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
		</CardHeader>
		<form onsubmit={saveAppLock}>
			<CardContent class="settings-card-content">
			<div class="setting-row">
				<div><Label for="app-lock-password">Password</Label><p>Saved passwords use the operating system credential store.</p></div>
				<div class="secret-field">
					<Input id="app-lock-password" class="settings-input settings-secret-input" type={appLockPasswordVisible ? "text" : "password"} bind:value={appLockPassword} autocomplete="new-password" autocapitalize="none" spellcheck="false" minlength={4} required />
					<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (appLockPasswordVisible = !appLockPasswordVisible)} aria-label={appLockPasswordVisible ? "Hide App Lock password" : "Show App Lock password"}>{#if appLockPasswordVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
				</div>
			</div>
			</CardContent>
			<CardFooter class="settings-card-footer">
				<span class="settings-footer-copy">This is a privacy screen, not content encryption.</span>
				<div class="lock-actions">
					{#if configuration?.appLock.status === "ready" && !configuration.appLockDev}<Button variant="ghost" size="sm" type="button" disabled={saving !== null} onclick={removeAppLock}>Remove</Button>{/if}
					<Button class="settings-save-button" size="sm" type="submit" disabled={saving !== null || appLockPassword.length < 4}>{saving === "app-lock" ? "Saving…" : "Save"}</Button>
				</div>
			</CardFooter>
		</form>
	</Card>

	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon"><KeyRound size={16} /></span>
			<div><CardTitle class="settings-card-title">UGOS</CardTitle><CardDescription class="settings-card-description">Connect to Task Manager through ugreen:9443.</CardDescription></div>
			{#if configuration !== null && configuration.ugos.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
		</CardHeader>
		<form onsubmit={saveUgos}>
			<CardContent class="settings-card-content">
			<div class="setting-row">
				<div><Label for="ugos-username">Username</Label><p>Your UGOS administrator account.</p></div>
				<Input id="ugos-username" class="settings-input" bind:value={username} autocomplete="username" required />
			</div>
			<div class="setting-row">
				<div><Label for="ugos-password">Password</Label><p>Stored in the operating system credential store.</p></div>
				<div class="secret-field">
					<Input id="ugos-password" class="settings-input settings-secret-input" type={passwordVisible ? "text" : "password"} bind:value={password} autocomplete="current-password" autocapitalize="none" spellcheck="false" required />
					<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (passwordVisible = !passwordVisible)} aria-label={passwordVisible ? "Hide password" : "Show password"}>{#if passwordVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
				</div>
			</div>
			</CardContent>
			<CardFooter class="settings-card-footer"><span class="settings-footer-copy">The certificate is trusted automatically on first connection.</span><Button class="settings-save-button" size="sm" type="submit" disabled={saving !== null}>{#if saving === "ugos"}Saving…{:else}Save{/if}</Button></CardFooter>
		</form>
	</Card>

	<Card>
		<CardHeader class="settings-card-header">
			<span class="icon"><KeyRound size={16} /></span>
			<div><CardTitle class="settings-card-title">Consumer APIs</CardTitle><CardDescription class="settings-card-description">Read metadata through each consumer's authenticated data boundary.</CardDescription></div>
		</CardHeader>
		<div class="api-settings">
			<div class="setting-row">
				<div><Label for="memos-api-key">my-memos API key</Label><p>Bearer key generated by the my-memos REST API settings.</p></div>
				<div class="api-input">
					<div class="secret-field">
						<Input id="memos-api-key" class="settings-input settings-secret-input" type={memosKeyVisible ? "text" : "password"} bind:value={memosApiKey} autocomplete="off" autocapitalize="none" spellcheck="false" placeholder="Bearer API key" />
						<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (memosKeyVisible = !memosKeyVisible)} aria-label={memosKeyVisible ? "Hide my-memos API key" : "Show my-memos API key"}>{#if memosKeyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
					</div>
					<Button type="button" size="sm" disabled={saving !== null || memosApiKey.trim() === ""} onclick={() => saveApi("memos")}>{saving === "memos" ? "Saving…" : "Save"}</Button>
					{#if configuration !== null && configuration.api.memos.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
				</div>
			</div>
			<div class="setting-row">
				<div><Label for="moment-api-key">my-moment API key</Label><p>Bearer key generated by the my-moment API settings.</p></div>
				<div class="api-input">
					<div class="secret-field">
						<Input id="moment-api-key" class="settings-input settings-secret-input" type={momentKeyVisible ? "text" : "password"} bind:value={momentApiKey} autocomplete="off" autocapitalize="none" spellcheck="false" placeholder="Bearer API key" />
						<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (momentKeyVisible = !momentKeyVisible)} aria-label={momentKeyVisible ? "Hide my-moment API key" : "Show my-moment API key"}>{#if momentKeyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
					</div>
					<Button type="button" size="sm" disabled={saving !== null || momentApiKey.trim() === ""} onclick={() => saveApi("moment")}>{saving === "moment" ? "Saving…" : "Save"}</Button>
					{#if configuration !== null && configuration.api.moment.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
				</div>
			</div>
			<div class="setting-row">
				<div><Label for="knowledge-api-key">my-knowledge API key</Label><p>Bearer key generated by the my-knowledge API settings.</p></div>
				<div class="api-input">
					<div class="secret-field">
						<Input id="knowledge-api-key" class="settings-input settings-secret-input" type={knowledgeKeyVisible ? "text" : "password"} bind:value={knowledgeApiKey} autocomplete="off" autocapitalize="none" spellcheck="false" placeholder="Bearer API key" />
						<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (knowledgeKeyVisible = !knowledgeKeyVisible)} aria-label={knowledgeKeyVisible ? "Hide my-knowledge API key" : "Show my-knowledge API key"}>{#if knowledgeKeyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
					</div>
					<Button type="button" size="sm" disabled={saving !== null || knowledgeApiKey.trim() === ""} onclick={() => saveApi("knowledge")}>{saving === "knowledge" ? "Saving…" : "Save"}</Button>
					{#if configuration !== null && configuration.api.knowledge.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
				</div>
			</div>
		</div>
	</Card>

	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon"><Cloud size={16} /></span>
			<div><CardTitle class="settings-card-title">Cloudflare R2</CardTitle><CardDescription class="settings-card-description">Read and publish content in cherry-studio.</CardDescription></div>
			{#if configuration !== null && configuration.r2.status === "ready"}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
		</CardHeader>
		<form onsubmit={saveR2}>
			<CardContent class="settings-card-content">
			<div class="setting-row">
				<div><Label for="r2-access-key">Access Key ID</Label><p>S3-compatible access key for this bucket.</p></div>
				<Input id="r2-access-key" class="settings-input" bind:value={accessKeyId} autocomplete="off" spellcheck="false" required />
			</div>
			<div class="setting-row">
				<div><Label for="r2-secret-key">Secret Access Key</Label><p>Kept in Keychain and never written to project files.</p></div>
				<div class="secret-field">
					<Input id="r2-secret-key" class="settings-input settings-secret-input" type={secretVisible ? "text" : "password"} bind:value={secretAccessKey} autocomplete="off" autocapitalize="none" spellcheck="false" required />
					<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (secretVisible = !secretVisible)} aria-label={secretVisible ? "Hide secret key" : "Show secret key"}>{#if secretVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
				</div>
			</div>
			</CardContent>
			<CardFooter class="settings-card-footer"><span class="settings-footer-copy">Changes apply to every R2-backed view immediately.</span><Button class="settings-save-button" size="sm" type="submit" disabled={saving !== null}>{#if saving === "r2"}Saving…{:else}Save{/if}</Button></CardFooter>
		</form>
	</Card>
	</div>
</section>

<style>
	.settings { width: min(100%, 64rem); margin: 0 auto; }
	header { margin-bottom: 1.5rem; }
	header p, header h1 { margin: 0; }
	header p { margin-bottom: 0.35rem; color: var(--color-accent); font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; }
	header h1 { font-family: var(--font-serif); font-size: 2rem; font-weight: 500; }
		.sections { display: grid; gap: 1rem; }
		:global(.settings-alert) { margin-bottom: 0.75rem; }
		:global(.settings-alert-copy) { font-size: 0.75rem; }
		:global(.settings-card-header) { display: grid; grid-template-columns: auto 1fr; align-items: center; gap: 0.75rem; padding: 1rem; }
		:global(.settings-card-header-status) { grid-template-columns: auto 1fr auto; }
		:global(.settings-card-title) { font-size: 0.82rem; }
		:global(.settings-card-description) { margin-top: 0.25rem; font-size: 0.68rem; }
		:global(.settings-card-content) { padding: 0; }
		:global(.settings-input) { height: 2rem; padding-inline: 0.625rem; font-size: 0.75rem; }
		:global(.settings-secret-input) { padding-right: 2rem; }
		.secret-field { position: relative; }
		:global(.settings-secret-toggle) { position: absolute; inset: 0 0 0 auto; width: 2rem; height: 2rem; }
		:global(.settings-card-footer) { min-height: 2.75rem; justify-content: space-between; padding: 0 1.25rem; border-top: 1px solid var(--color-border); background: var(--color-muted); }
		.settings-footer-copy { color: var(--color-muted-foreground); font-size: 0.68rem; }
		:global(.settings-save-button) { height: 1.75rem; padding-inline: 0.625rem; font-size: 0.68rem; font-weight: 400; }
	.icon { display: grid; width: 2rem; height: 2rem; place-items: center; border-radius: var(--radius-md); background: var(--color-muted); color: var(--color-muted-foreground); }
	.setting-row p { margin: 0.15rem 0 0; color: var(--color-muted-foreground); font-size: 0.68rem; }
	.configured { display: inline-flex; align-items: center; gap: 0.3rem; color: var(--color-success); font-size: 0.68rem; }
	form { border-top: 1px solid var(--color-border); }
	.setting-row { display: grid; grid-template-columns: minmax(10rem, 0.8fr) minmax(16rem, 1.2fr); align-items: center; gap: 2rem; padding: 0.8rem 1.25rem; }
	.setting-row + .setting-row { border-top: 1px solid var(--color-border); }
	.setting-row :global(label) { font-size: 0.72rem; }
	.api-settings { border-top: 1px solid var(--color-border); }
	.api-input { display: grid; grid-template-columns: minmax(12rem, 1fr) auto auto; align-items: center; gap: 0.5rem; }
	.lock-actions { display: flex; align-items: center; gap: 0.4rem; }
		@media (max-width: 640px) {
		.setting-row { grid-template-columns: 1fr; gap: 0.5rem; }
		.api-input { grid-template-columns: 1fr auto; }
			.api-input .configured { grid-column: 1 / -1; }
			.settings-footer-copy { display: none; }
	}
</style>
