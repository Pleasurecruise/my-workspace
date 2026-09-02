<script lang="ts">
	import { BellRing, CircleCheck, Cloud, Eye, EyeOff, KeyRound, LoaderCircle, Lock, Music2, QrCode, Send, X } from "@lucide/svelte";
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
	import { invoke } from "@tauri-apps/api/core";
	import { onDestroy, tick } from "svelte";
	import type {
		ApiConfiguration,
		CommandResponse,
		ConfigurationStatus,
		NtfyConfig,
		QqLoginStatus,
		QqQr,
		R2Configuration,
		TelegramAuthorizationStatus,
		TelegramCredentials,
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
		onconnectspotify,
		onbeginqq,
		onpollqq,
		oncancelqq,
		onconfigurationchanged,
	}: {
		configuration: ConfigurationStatus | null;
		error: string | null;
		onsaveugos: (input: UgosConfiguration) => Promise<CommandResponse<string>>;
		onsaver2: (input: R2Configuration) => Promise<CommandResponse<string>>;
		onsaveapi: (input: ApiConfiguration) => Promise<CommandResponse<string>>;
		onsaventfy: (configuration: NtfyConfig) => Promise<CommandResponse<string>>;
		onsaveapplock: (password: string) => Promise<CommandResponse<string>>;
		onremoveapplock: () => Promise<CommandResponse<string>>;
		onconnectspotify: () => Promise<CommandResponse<string>>;
		onbeginqq: () => Promise<CommandResponse<QqQr>>;
		onpollqq: () => Promise<CommandResponse<QqLoginStatus>>;
		oncancelqq: () => Promise<CommandResponse<null>>;
		onconfigurationchanged: () => Promise<void>;
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
	let saving = $state<"app-lock" | "ugos" | "r2" | "memos" | "moment" | "knowledge" | "ntfy" | "spotify" | "qqMusic" | "telegram" | "telegram-auth" | "x" | null>(null);
	let qqQr = $state<QqQr | null>(null);
	let qqDialog = $state<HTMLDivElement | null>(null);
	let qqStatus = $state<"waiting" | "scanned" | "expired">("waiting");
	let qqLoginGeneration = 0;
	let formError = $state<string | null>(null);
	let passwordVisible = $state(false);
	let secretVisible = $state(false);
	let memosKeyVisible = $state(false);
	let momentKeyVisible = $state(false);
	let knowledgeKeyVisible = $state(false);
	let ntfyTokenVisible = $state(false);
	let appLockPasswordVisible = $state(false);
	let telegramApiId = $state("");
	let telegramApiHash = $state("");
	let telegramChannel = $state("");
	let telegramPhone = $state("");
	let telegramCode = $state("");
	let telegramPassword = $state("");
	let telegramApiHashVisible = $state(false);
	let telegramPasswordVisible = $state(false);
	let telegramAuthorization = $state<TelegramAuthorizationStatus | null>(null);
	let telegramAuthorizationChecked = false;

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

	$effect(() => {
		if (configuration?.publication.telegram !== true) {
			telegramAuthorization = null;
			telegramAuthorizationChecked = false;
			return;
		}
		if (telegramAuthorizationChecked) return;
		telegramAuthorizationChecked = true;
		void invoke<CommandResponse<TelegramAuthorizationStatus>>("read_auth").then((response) => {
			if (response.status === "ready") telegramAuthorization = response.data;
			else formError = response.message;
		});
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

	async function saveTelegram(event: SubmitEvent) {
		event.preventDefault();
		const apiId = Number(telegramApiId);
		if (!Number.isInteger(apiId) || apiId <= 0) {
			formError = "Telegram API ID must be a positive integer.";
			return;
		}
		saving = "telegram";
		formError = null;
		const credentials: TelegramCredentials = {
			apiId,
			apiHash: telegramApiHash,
			channelUsername: telegramChannel,
		};
		const response = await invoke<CommandResponse<string>>("save_telegram", { credentials });
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		telegramApiHash = "";
		telegramApiHashVisible = false;
		telegramAuthorizationChecked = true;
		await onconfigurationchanged();
		const authorization = await invoke<CommandResponse<TelegramAuthorizationStatus>>("read_auth");
		if (authorization.status === "ready") telegramAuthorization = authorization.data;
		else formError = authorization.message;
	}

	async function continueTelegramAuth(event: SubmitEvent) {
		event.preventDefault();
		saving = "telegram-auth";
		formError = null;
		let response: CommandResponse<TelegramAuthorizationStatus>;
		if (telegramAuthorization?.status === "codeRequired") {
			response = await invoke("submit_code", { code: telegramCode });
		} else if (telegramAuthorization?.status === "passwordRequired") {
			response = await invoke("submit_password", { password: telegramPassword });
		} else {
			response = await invoke("begin_auth", { phone: telegramPhone });
		}
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		telegramAuthorization = response.data;
		if (response.data.status === "ready") {
			telegramCode = "";
			telegramPassword = "";
			telegramPasswordVisible = false;
		}
	}

	async function cancelTelegramAuth() {
		if (saving !== null) return;
		saving = "telegram-auth";
		formError = null;
		const response = await invoke<CommandResponse<string>>("cancel_auth");
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		telegramAuthorization = { status: "disconnected" };
		telegramCode = "";
		telegramPassword = "";
	}

	async function connectX() {
		saving = "x";
		formError = null;
		const response = await invoke<CommandResponse<string>>("connect_x");
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		await onconfigurationchanged();
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

	async function connectSpotify() {
		saving = "spotify";
		formError = null;
		const response = await onconnectspotify();
		saving = null;
		if (response.status === "failed") formError = response.message;
	}

	async function connectQqMusic() {
		saving = "qqMusic";
		formError = null;
		const response = await onbeginqq();
		saving = null;
		if (response.status === "failed") {
			formError = response.message;
			return;
		}
		qqQr = response.data;
		qqStatus = "waiting";
		const generation = ++qqLoginGeneration;
		await tick();
		qqDialog?.focus();
		void pollQqMusic(generation);
	}

	async function pollQqMusic(generation: number) {
		while (qqQr !== null && generation === qqLoginGeneration) {
			await new Promise((resolve) => window.setTimeout(resolve, 1_500));
			if (qqQr === null || generation !== qqLoginGeneration) return;
			const response = await onpollqq();
			if (response.status === "failed") {
				formError = response.message;
				qqQr = null;
				return;
			}
			if (response.data.status === "complete") {
				qqQr = null;
				return;
			}
			qqStatus = response.data.status;
			if (response.data.status === "expired") return;
		}
	}

	function closeQqLogin() {
		qqLoginGeneration += 1;
		qqQr = null;
		void oncancelqq();
	}

	onDestroy(() => {
		if (qqQr === null) return;
		qqLoginGeneration += 1;
		qqQr = null;
		void oncancelqq();
	});
</script>

<svelte:window onkeydown={(event) => { if (event.key === "Escape" && qqQr !== null) closeQqLogin(); }} />

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
			<span class="icon"><Music2 size={16} /></span>
			<div><CardTitle class="settings-card-title">Spotify Music</CardTitle><CardDescription class="settings-card-description">Read Liked Songs and control playback through your Spotify account.</CardDescription></div>
			{#if configuration?.spotify.status === "ready"}<span class="configured"><CircleCheck size={14} /> Connected</span>{/if}
		</CardHeader>
		<CardContent class="settings-card-content">
			<div class="setting-row">
				<div><Label>Shared authorization</Label><p>No developer application or Client Secret is required. Spotify opens two browser grants: one for Liked Songs and one for local playback.</p></div>
				<Button class="settings-save-button" size="sm" type="button" disabled={saving !== null} onclick={connectSpotify}>{saving === "spotify" ? "Waiting for Spotify…" : configuration?.spotify.status === "ready" ? "Reconnect" : "Connect"}</Button>
			</div>
		</CardContent>
		<CardFooter class="settings-card-footer"><span class="settings-footer-copy">Refresh grants stay in the operating-system credential store. Local playback requires Spotify Premium.</span></CardFooter>
	</Card>

	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon"><Music2 size={16} /></span>
			<div><CardTitle class="settings-card-title">QQ Music</CardTitle><CardDescription class="settings-card-description">Play the personalized Daily 30 recommendation from your QQ Music account.</CardDescription></div>
			{#if configuration?.qqMusic.status === "ready"}<span class="configured"><CircleCheck size={14} /> Connected</span>{/if}
		</CardHeader>
		<CardContent class="settings-card-content">
			<div class="setting-row">
				<div><Label>QR authorization</Label><p>Scan with the QQ mobile app. Vesper stores the resulting renewable session automatically.</p></div>
				<Button class="settings-save-button" size="sm" type="button" disabled={saving !== null} onclick={connectQqMusic}>{saving === "qqMusic" ? "Creating QR code…" : configuration?.qqMusic.status === "ready" ? "Reconnect" : "Connect"}</Button>
			</div>
		</CardContent>
		<CardFooter class="settings-card-footer"><span class="settings-footer-copy">Login credentials stay in the operating-system credential store and renew automatically.</span></CardFooter>
	</Card>

	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon"><Send size={16} /></span>
			<div><CardTitle class="settings-card-title">Telegram Channel</CardTitle><CardDescription class="settings-card-description">Publish public Memos through an authorized Telegram user account.</CardDescription></div>
			{#if configuration?.publication.telegram}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
		</CardHeader>
		<form onsubmit={saveTelegram}>
			<CardContent class="settings-card-content">
				<div class="setting-row">
					<div><Label for="telegram-api-id">API ID</Label><p>Numeric application ID from my.telegram.org.</p></div>
					<Input id="telegram-api-id" class="settings-input" bind:value={telegramApiId} inputmode="numeric" autocomplete="off" required />
				</div>
				<div class="setting-row">
					<div><Label for="telegram-api-hash">API Hash</Label><p>The 32-character application hash.</p></div>
					<div class="secret-field">
						<Input id="telegram-api-hash" class="settings-input settings-secret-input" type={telegramApiHashVisible ? "text" : "password"} bind:value={telegramApiHash} autocomplete="off" autocapitalize="none" spellcheck="false" minlength={32} maxlength={32} required />
						<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (telegramApiHashVisible = !telegramApiHashVisible)} aria-label={telegramApiHashVisible ? "Hide Telegram API hash" : "Show Telegram API hash"}>{#if telegramApiHashVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
					</div>
				</div>
				<div class="setting-row">
					<div><Label for="telegram-channel">Channel username</Label><p>Public channel username; the signed-in account must be allowed to post.</p></div>
					<Input id="telegram-channel" class="settings-input" bind:value={telegramChannel} autocomplete="off" autocapitalize="none" spellcheck="false" placeholder="channel_username" required />
				</div>
			</CardContent>
			<CardFooter class="settings-card-footer"><span class="settings-footer-copy">Save credentials before authorizing the Telegram account.</span><Button class="settings-save-button" size="sm" type="submit" disabled={saving !== null || telegramApiId.trim() === "" || telegramApiHash.length !== 32 || telegramChannel.trim() === ""}>{saving === "telegram" ? "Saving…" : "Save"}</Button></CardFooter>
		</form>
		{#if configuration?.publication.telegram}
			{#if telegramAuthorization?.status === "ready"}
				<div class="setting-row telegram-authorization"><div><Label>User authorization</Label><p>The local MTProto session is ready to publish.</p></div><span class="configured"><CircleCheck size={14} /> Authorized</span></div>
			{:else}
				<form onsubmit={continueTelegramAuth}>
					<CardContent class="settings-card-content">
						<div class="setting-row telegram-authorization">
							{#if telegramAuthorization?.status === "codeRequired"}
								<div><Label for="telegram-code">Verification code</Label><p>Enter the code sent by Telegram.</p></div>
								<Input id="telegram-code" class="settings-input" bind:value={telegramCode} inputmode="numeric" autocomplete="one-time-code" required />
							{:else if telegramAuthorization?.status === "passwordRequired"}
								<div><Label for="telegram-password">2FA password</Label><p>{telegramAuthorization.hint === null ? "Enter the Telegram account password." : `Hint: ${telegramAuthorization.hint}`}</p></div>
								<div class="secret-field">
									<Input id="telegram-password" class="settings-input settings-secret-input" type={telegramPasswordVisible ? "text" : "password"} bind:value={telegramPassword} autocomplete="current-password" required />
									<Button type="button" class="settings-secret-toggle" variant="ghost" size="icon" onclick={() => (telegramPasswordVisible = !telegramPasswordVisible)} aria-label={telegramPasswordVisible ? "Hide Telegram password" : "Show Telegram password"}>{#if telegramPasswordVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</Button>
								</div>
							{:else}
								<div><Label for="telegram-phone">Phone number</Label><p>Use the number attached to the Telegram account.</p></div>
								<Input id="telegram-phone" class="settings-input" bind:value={telegramPhone} inputmode="tel" autocomplete="tel" placeholder="+86…" required />
							{/if}
						</div>
					</CardContent>
					<CardFooter class="settings-card-footer">
						<span class="settings-footer-copy">The authorization session remains on this device.</span>
						<div class="lock-actions">
							{#if telegramAuthorization?.status === "codeRequired" || telegramAuthorization?.status === "passwordRequired"}<Button variant="ghost" size="sm" type="button" disabled={saving !== null} onclick={cancelTelegramAuth}>Cancel</Button>{/if}
							<Button class="settings-save-button" size="sm" type="submit" disabled={saving !== null}>{saving === "telegram-auth" ? "Waiting…" : telegramAuthorization?.status === "codeRequired" ? "Verify code" : telegramAuthorization?.status === "passwordRequired" ? "Verify password" : "Send code"}</Button>
						</div>
					</CardFooter>
				</form>
			{/if}
		{/if}
	</Card>

	<Card>
		<CardHeader class="settings-card-header settings-card-header-status">
			<span class="icon">{@render XLogo()}</span>
			<div><CardTitle class="settings-card-title">X / Twitter</CardTitle><CardDescription class="settings-card-description">Publish public Memos through the X user-context API.</CardDescription></div>
			{#if configuration?.publication.x}<span class="configured"><CircleCheck size={14} /> Configured</span>{/if}
		</CardHeader>
		<CardContent class="settings-card-content">
			<div class="setting-row">
				<div><Label>Browser authorization</Label><p>Vesper opens X and requests permission to publish from your account.</p></div>
				<Button class="settings-save-button" size="sm" type="button" disabled={saving !== null} onclick={connectX}>{saving === "x" ? "Waiting for X…" : configuration?.publication.x ? "Reconnect" : "Connect"}</Button>
			</div>
		</CardContent>
		<CardFooter class="settings-card-footer"><span class="settings-footer-copy">Access and refresh grants stay in the operating-system credential store and renew automatically.</span></CardFooter>
	</Card>

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

{#snippet XLogo()}
	<svg aria-hidden="true" viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" /></svg>
{/snippet}

{#if qqQr !== null}
	<div class="qq-login-backdrop" role="presentation" onclick={(event) => { if (event.currentTarget === event.target) closeQqLogin(); }}>
		<div bind:this={qqDialog} class="qq-login" role="dialog" aria-modal="true" aria-labelledby="qq-login-title" tabindex="-1">
			<Button class="qq-login-close" variant="ghost" size="icon" type="button" onclick={closeQqLogin} aria-label="Close QQ Music login"><X size={16} /></Button>
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
	.qq-login-backdrop { position: fixed; inset: 0; z-index: 100; display: grid; place-items: center; box-sizing: border-box; padding: 1rem; background: var(--color-overlay); backdrop-filter: blur(10px); }
	.qq-login { position: relative; display: grid; width: min(100%, 22rem); justify-items: center; box-sizing: border-box; padding: 1.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-lg); text-align: center; }
	:global(.qq-login-close) { position: absolute; top: 0.75rem; right: 0.75rem; }
	.qq-login-icon { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-accent); }
	.qq-login h2 { margin: 0.75rem 0 0; font-family: var(--font-serif); font-size: 1.25rem; font-weight: 500; }
	.qq-login > p { max-width: 17rem; margin: 0.4rem 0 1rem; color: var(--color-muted-foreground); font-size: 0.75rem; line-height: 1.5; }
	.qq-code { display: grid; width: 12rem; height: 12rem; place-items: center; padding: 0.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); transition: opacity var(--duration-normal); }
	.qq-code.expired { opacity: 0.28; }
	.qq-code img { display: block; width: 100%; height: 100%; image-rendering: pixelated; }
	.qq-login-status { display: flex; min-height: 1.25rem; align-items: center; gap: 0.4rem; margin-top: 1rem; color: var(--color-muted-foreground); font-size: 0.7rem; }
	:global(.qq-login-spinner) { animation: qq-login-spin var(--duration-slow) linear infinite; }
	@keyframes qq-login-spin { to { rotate: 360deg; } }
	@media (max-width: 640px) {
		.setting-row { grid-template-columns: 1fr; gap: 0.5rem; }
		.api-input { grid-template-columns: 1fr auto; }
		.api-input .configured { grid-column: 1 / -1; }
		.settings-footer-copy { display: none; }
	}
</style>
