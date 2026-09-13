<script lang="ts">
	import "./lib/components/layout/page.css";
	import ProfileEditor from "./lib/components/layout/ProfileEditor.svelte";
	import UpdateDialog from "./lib/components/layout/UpdateDialog.svelte";
	import PageSkeleton from "./lib/components/layout/PageSkeleton.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { Archive, ArrowLeft, Bell, BookOpen, CloudOff, Heart, Home, Image, LayoutDashboard, Lock, Menu, Moon, Music2, Newspaper as NewspaperIcon, Settings, Sun, X } from "@lucide/svelte";
	import { onMount, tick, untrack } from "svelte";
	import MemosView from "./lib/components/pages/MemosView.svelte";
	import MomentView from "./lib/components/pages/MomentView.svelte";
	import MusicView from "./lib/components/pages/MusicView.svelte";
	import KnowledgeView, { selectKnowledgeArticle } from "./lib/components/pages/KnowledgeView.svelte";
	import InboxView from "./lib/components/pages/InboxView.svelte";
	import NewspaperView from "./lib/components/pages/NewspaperView.svelte";
	import { createLayoutSession } from "./lib/components/dashboard/layout.svelte";
	import DashboardView from "./lib/components/pages/DashboardView.svelte";
	import SettingsView from "./lib/components/pages/SettingsView.svelte";
	import ScrollToTop from "./lib/components/layout/ScrollToTop.svelte";
	import type {
		CommandResponse,
		Channel,
		InitialViews,
	} from "./lib/consumer";
	import { createDashboardSession } from "./lib/components/dashboard/session.svelte";
	import { createInboxSession } from "./lib/components/inbox/session.svelte";
	import { createSettingsSession } from "./lib/components/settings/session.svelte";
	import { createMemosSession } from "./lib/components/memos/session.svelte";
	import { createMomentSession } from "./lib/components/moment/session.svelte";
	import { createKnowledgeSession } from "./lib/components/knowledge/session.svelte";
	import { applyTheme, initTheme } from "./lib/theme";

	type View = "dashboard" | "inbox" | "music" | "newspaper" | "settings" | Channel;

	const navigation: Array<{ id: View; label: string }> = [
		{ id: "dashboard", label: "Dashboard" },
		{ id: "newspaper", label: "Newspaper" },
		{ id: "memos", label: "Memos" },
		{ id: "moment", label: "Moment" },
		{ id: "music", label: "Music" },
		{ id: "knowledge", label: "Knowledge" },
		{ id: "settings", label: "Settings" },
	];
	const sidebarWidthKey = "vesper.sidebar.width";
	const minimumSidebarWidth = 64;
	const maximumSidebarWidth = 360;

	let contentWidth = $state(0);
	let selected = $state<View>("dashboard");
	let reconnectMihoyo = $state(false);
	let musicPlayerVisible = $state(false);
	let musicPlayerAvailable = $state(false);
	let musicReturnView: View = "music";
	let previousView = $state<Exclude<View, "inbox">>("dashboard");
	let mainElement = $state<HTMLElement | null>(null);
	const memos = createMemosSession({
		get active() { return selected === "memos"; },
		get mainElement() { return mainElement; },
	});
	const moment = createMomentSession({
		get active() { return selected === "moment"; },
		get mainElement() { return mainElement; },
	});
	const knowledge = createKnowledgeSession({
		get active() { return selected === "knowledge" || selected === "newspaper"; },
		get mainElement() { return mainElement; },
	});
	const activeContent = $derived(
		selected === "memos" ? memos : selected === "moment" ? moment
			: selected === "knowledge" || selected === "newspaper" ? knowledge : null,
	);
	const paginatedContent = $derived(
		selected === "memos" ? memos : selected === "knowledge" ? knowledge : null,
	);
	const content = $derived(activeContent === null ? null : activeContent.content);
	const contentError = $derived(activeContent === null ? null : activeContent.error);
	let initializationRequest = 0;
	let dark = $state(initTheme());
	let sidebarOpen = $state(false);
	const layoutSession = createLayoutSession();
	const dashboardSession = createDashboardSession(() => selected === "dashboard");
	const inbox = createInboxSession(() => selected === "inbox");
	const settings = createSettingsSession({
		resetChannel: (channel) => {
			if (channel === "memos") memos.reset();
			else if (channel === "moment") moment.reset();
			else knowledge.reset();
		},
		initializeConsumers,
		refreshDashboard: dashboardSession.refreshDashboard,
	});
	function viewAvailable(view: View): boolean {
		const configuration = settings.configuration;
		switch (view) {
			case "memos": return configuration?.api.memos.status === "ready";
			case "moment": return configuration?.api.moment.status === "ready";
			case "knowledge":
			case "newspaper": return configuration?.api.knowledge.status === "ready";
			case "music": return configuration?.spotify.status === "ready" || configuration?.qqMusic.status === "ready";
			case "inbox": return configuration?.ntfy.status === "ready";
			default: return true;
		}
	}
	const visibleNavigation = $derived(navigation.filter((item) => viewAvailable(item.id)));
	$effect(() => {
		if (!viewAvailable(selected)) untrack(() => { void select("dashboard"); });
	});
	let updateModalOpen = $state(false);
	let locked = $state(true);
	let unlockPassword = $state("");
	let unlockError = $state<string | null>(null);
	let unlocking = $state(false);
	let unlockInput = $state<HTMLInputElement | null>(null);
	let sidebarWidth = $state(240);

	function openMusicPlayer() {
		musicReturnView = selected;
		void select("music");
		musicPlayerVisible = true;
	}

	async function select(view: View) {
		reconnectMihoyo = false;
		if (view === "inbox") {
			if (selected === "inbox") view = previousView;
			else previousView = selected;
		}
		if (!viewAvailable(view)) view = "dashboard";
		activeContent?.leave();
		musicPlayerVisible = false;
		selected = view;
		void inbox.activate(view === "inbox");
		const activation = dashboardSession.activate(view === "dashboard");
		sidebarOpen = false;
		await activeContent?.enter(view === "newspaper");
		if (view === "dashboard") await activation;
	}

	async function lockApp() {
		if (settings.configuration?.appLock.status !== "ready") {
			await select("settings");
			return;
		}
		locked = true;
		const response = await invoke<CommandResponse<null>>("lock_app");
		if (response.status === "failed") {
			locked = false;
			settings.error = response.message;
			await select("settings");
			return;
		}
		sidebarOpen = false;
		unlockPassword = "";
		unlockError = null;
		await tick();
		unlockInput?.focus();
	}

	async function unlockApp(event: SubmitEvent) {
		event.preventDefault();
		if (unlocking || unlockPassword === "") return;
		unlocking = true;
		unlockError = null;
		const response = await invoke<CommandResponse<string>>("unlock_app", { password: unlockPassword });
		unlocking = false;
		if (response.status === "failed") {
			unlockError = response.message;
			unlockPassword = "";
			await tick();
			unlockInput?.focus();
			return;
		}
		unlockPassword = "";
		locked = false;
		void Promise.all([memos.refresh(), moment.refresh(), knowledge.refresh()]);
	}

	function loadSidebarWidth() {
		const savedSidebarWidth = Number(localStorage.getItem(sidebarWidthKey));
		if (Number.isFinite(savedSidebarWidth) && savedSidebarWidth >= minimumSidebarWidth && savedSidebarWidth <= maximumSidebarWidth) {
			sidebarWidth = savedSidebarWidth;
		}
	}

	function beginSidebarResize(event: PointerEvent & { currentTarget: HTMLButtonElement }) {
		if (window.innerWidth < 768) return;
		const handle = event.currentTarget;
		const pointerId = event.pointerId;
		const startingX = event.clientX;
		const startingWidth = sidebarWidth;
		handle.setPointerCapture(pointerId);

		const resize = (moveEvent: PointerEvent) => {
			sidebarWidth = Math.min(
				maximumSidebarWidth,
				Math.max(minimumSidebarWidth, startingWidth + moveEvent.clientX - startingX),
			);
		};
		const finish = () => {
			handle.removeEventListener("pointermove", resize);
			handle.removeEventListener("pointerup", finish);
			handle.removeEventListener("pointercancel", finish);
			if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
			localStorage.setItem(sidebarWidthKey, String(sidebarWidth));
		};

		handle.addEventListener("pointermove", resize);
		handle.addEventListener("pointerup", finish);
		handle.addEventListener("pointercancel", finish);
	}

	function resizeSidebarByKey(event: KeyboardEvent) {
		if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
		event.preventDefault();
		const delta = event.key === "ArrowLeft" ? -8 : 8;
		if (event.key === "Home") sidebarWidth = minimumSidebarWidth;
		else if (event.key === "End") sidebarWidth = maximumSidebarWidth;
		else sidebarWidth = Math.min(maximumSidebarWidth, Math.max(minimumSidebarWidth, sidebarWidth + delta));
		localStorage.setItem(sidebarWidthKey, String(sidebarWidth));
	}

	function toggleTheme() {
		dark = !dark;
		applyTheme(dark);
	}


	async function initializeConsumers() {
		const version = ++initializationRequest;
		const versions = { memos: memos.version, moment: moment.version, knowledge: knowledge.version };
		const initial = await invoke<InitialViews>("initialize_views");
		if (version !== initializationRequest) return;
		memos.initialize(initial.memos, versions.memos);
		moment.initialize(initial.moment, versions.moment);
		knowledge.initialize(initial.knowledge, versions.knowledge);
	}

	onMount(() => {
		void initializeConsumers();
		void invoke<boolean>("read_app_lock").then(async (value) => {
			locked = value;
			if (value) {
				await tick();
				unlockInput?.focus();
			}
		});
		loadSidebarWidth();
		void settings.loadConfiguration();
		const unlistenGameLogin = listen("game-login-required", () => {
			void select("settings").then(() => { reconnectMihoyo = true; });
		});
		return () => {
			void unlistenGameLogin.then((unlisten) => unlisten());
		};
	});
</script>

<svelte:head>
	<title>Vesper</title>
	<meta name="description" content="Local previews for Memos and Moment." />
</svelte:head>

<div class="shell" class:locked inert={locked || updateModalOpen} style:--sidebar-width={`${sidebarWidth}px`}>
	<button
		type="button"
		class:open={sidebarOpen}
		class="sidebar-overlay"
		onclick={() => (sidebarOpen = false)}
		aria-label="Close sidebar"
	></button>

	<aside class:open={sidebarOpen} class:compact={sidebarWidth < 160}>
		<div class="sidebar-header">
			<button type="button" class="close-sidebar" onclick={() => (sidebarOpen = false)} aria-label="Close sidebar">
				<X size={15} />
			</button>
		</div>

		<nav aria-label="Consumer views">
			{#each visibleNavigation as item}
				<button
					type="button"
					class:active={selected === item.id}
					aria-current={selected === item.id ? "page" : "false"}
					aria-label={item.label}
					title={item.label}
					onclick={() => void select(item.id)}
				>
					{#if item.id === "dashboard"}<LayoutDashboard size={15} />{:else if item.id === "memos"}<Home size={15} />{:else if item.id === "moment"}<Image size={15} />{:else if item.id === "music"}<Music2 size={15} />{:else if item.id === "newspaper"}<NewspaperIcon size={15} />{:else if item.id === "knowledge"}<BookOpen size={15} />{:else}<Settings size={15} />{/if}
					<span>{item.label}</span>
				</button>
			{/each}
		</nav>

		<div class="sidebar-footer">
			<ProfileEditor compact={sidebarWidth < 160} />
			<div class="footer-controls">
				{#if viewAvailable("inbox")}
				<div class="footer-navigation">
					<button
						class:active={selected === "inbox"}
						type="button"
						onclick={() => void select("inbox")}
						aria-label={selected === "inbox"
							? "Return to previous view"
							: inbox.notifications.length > 0
								? `Open inbox, ${inbox.notifications.length} unread notifications`
								: "Open inbox"}
						title={selected === "inbox" ? "Back" : "Inbox"}
					>
						<Bell size={15} />
						{#if inbox.notifications.length > 0}<span class="notification-dot" aria-hidden="true"></span>{/if}
					</button>
				</div>
				{/if}
				<div class="footer-actions">
					<button type="button" onclick={lockApp} aria-label={settings.configuration?.appLock.status === "ready" ? "Lock Vesper" : "Configure App Lock"} title={settings.configuration?.appLock.status === "ready" ? "Lock Vesper" : "Configure App Lock in Settings"}>
						<Lock size={15} />
					</button>
					<button type="button" onclick={toggleTheme} aria-label={dark ? "Switch to light mode" : "Switch to dark mode"} title={dark ? "Light mode" : "Dark mode"}>
						{#if dark}<Sun size={15} />{:else}<Moon size={15} />{/if}
					</button>
				</div>
			</div>
		</div>
		<button
			type="button"
			class="sidebar-resizer"
			aria-label={`Resize sidebar, currently ${sidebarWidth} pixels wide`}
			onpointerdown={beginSidebarResize}
			onkeydown={resizeSidebarByKey}
		></button>
	</aside>

	<main
		class:player-shell={selected === "music" && musicPlayerVisible}
		bind:this={mainElement}
		onscroll={() => {
			if (
				mainElement !== null &&
				mainElement.scrollHeight - mainElement.scrollTop - mainElement.clientHeight < 600
			) paginatedContent?.loadMore(true);
		}}
	>
		<header class="topbar">
			<button class="menu-button" type="button" onclick={() => (sidebarOpen = true)} aria-label="Open sidebar">
				<Menu size={18} />
			</button>
			<strong>vesper</strong>
		</header>
		<div class="canvas page-layout">
			<div class="page-content" bind:clientWidth={contentWidth} data-stacked={contentWidth <= 640}>
				{#if selected === "dashboard"}
					<DashboardView session={dashboardSession} {layoutSession} />
				{:else if selected === "settings"}
					<SettingsView
						{reconnectMihoyo}
						configuration={settings.configuration}
						error={settings.error}
						onsaveugos={settings.saveUgosConfiguration}
						onsaver2={settings.saveR2Configuration}
						onsaveapi={settings.saveApiConfiguration}
						onsaventfy={settings.saveNtfy}
						onsavenotion={settings.saveNotionCalendar}
						onsavecodexresets={settings.saveCodexResets}
						onsaveapplock={settings.saveAppLock}
						onremoveapplock={settings.removeAppLock}
						onconnectspotify={settings.connectSpotify}
						onconfigurationchanged={settings.loadConfiguration}
					/>
				{:else if selected === "inbox"}
					<InboxView notifications={inbox.notifications} loadError={inbox.error} onread={inbox.markNotificationRead} />
				{:else if selected === "music"}
					{#key settings.spotifyRevision}
						<MusicView spotifyRevision={settings.spotifyRevision} bind:playerVisible={musicPlayerVisible} bind:playerAvailable={musicPlayerAvailable} onopenplayer={openMusicPlayer} onopensettings={() => void select("settings")} />
					{/key}
				{:else if contentError && content === null}
					<section class="consumer-error">
						<header class="page-header"><div>
							{#if selected === "memos"}<h1>Memos</h1>{:else if selected === "moment"}<h1>Moment</h1>{:else if selected === "newspaper"}<h1>Newspaper</h1>{:else}<h1>Knowledge</h1>{/if}
							<p class="page-description">Content unavailable</p>
						</div></header>
						<div class="error" role="alert">
							<CloudOff size={18} />
							<div><strong>Content unavailable</strong><span>{contentError}</span></div>
							<button type="button" onclick={() => void select("settings")}>Open Settings</button>
						</div>
					</section>
				{:else if selected === "newspaper"}
					<NewspaperView onread={knowledge.readArticle} documents={knowledge.content?.knowledge ?? []} issues={knowledge.content?.newspaper ?? { developer: null, personal: null }} loading={knowledge.content === null || knowledge.loading} onopenarticle={(document, fragment) => { const error = selectKnowledgeArticle(document, fragment); if (error === null) selected = "knowledge"; return error; }} />
				{:else if content !== null}
					{#if content.channel === "memos"}
						<MemosView memos={content.memos} tags={memos.tags.tags} display={memos.memoDisplay} onfilter={memos.filterMemos} onopenmemo={memos.revealMemo} oncreate={memos.createMemo} onimportx={memos.importXMemo} onupdate={memos.updateMemo} ondelete={memos.deleteMemo} onpublishtelegram={memos.publishMemoToTelegram} onpublishx={memos.publishMemoToX}>
							{#snippet tagStatus()}
						{#if memos.tags.error}
							<div class="tag-notice" role="alert"><span>Tags unavailable: {memos.tags.error}</span><button type="button" disabled={memos.tags.loading} onclick={() => void memos.tags.refresh()}>Retry tags</button></div>
						{:else if memos.tags.loading && memos.tags.tags.length === 0}
							<p class="tag-status" role="status">Loading tags…</p>
						{/if}
							{/snippet}
						</MemosView>
					{:else if content.channel === "moment"}
						<MomentView photos={content.photos} tags={moment.tags.tags} total={content.total} onupload={moment.createPhoto} onupdate={moment.updatePhoto} ondelete={moment.deletePhoto}>
							{#snippet tagStatus()}
						{#if moment.tags.error}
							<div class="tag-notice" role="alert"><span>Tags unavailable: {moment.tags.error}</span><button type="button" disabled={moment.tags.loading} onclick={() => void moment.tags.refresh()}>Retry tags</button></div>
						{:else if moment.tags.loading && moment.tags.tags.length === 0}
							<p class="tag-status" role="status">Loading tags…</p>
						{/if}
							{/snippet}
						</MomentView>
					{:else}
						<KnowledgeView onread={knowledge.readArticle} documents={content.knowledge} loading={knowledge.loading} oncreate={knowledge.createKnowledge} onupdate={knowledge.updateKnowledge} />
					{/if}
				{:else}
					<PageSkeleton view={selected === "moment" ? "moment" : selected === "knowledge" ? "knowledge" : "memos"} title={selected === "moment" ? "Moment" : selected === "knowledge" ? "Knowledge" : "Memos"} />
				{/if}
				<div class="sentinel" aria-hidden="true"></div>
				{#if contentError && content !== null}
					<p class="tag-notice" role="alert">{contentError}</p>
				{/if}
				{#if paginatedContent?.loadingMore && content}
					<p class="loading-more">Loading more…</p>
				{/if}
			</div>
		</div>
		<div class="global-scroll-action">
			{#if selected === "memos"}
				<button
					class="memo-filter-action"
					class:active={memos.memoDisplay === "archived"}
					type="button"
					onclick={() => (memos.memoDisplay = memos.memoDisplay === "archived" ? "active" : "archived")}
					aria-pressed={memos.memoDisplay === "archived"}
					aria-label={memos.memoDisplay === "archived" ? "Show active memos" : "Show archived memos"}
					title={memos.memoDisplay === "archived" ? "Active memos" : "Archived memos"}
				>
					<Archive size={15} />
				</button>
				<button
					class="memo-filter-action"
					class:active={memos.memoDisplay === "favorites"}
					type="button"
					onclick={() => (memos.memoDisplay = memos.memoDisplay === "favorites" ? "active" : "favorites")}
					aria-pressed={memos.memoDisplay === "favorites"}
					aria-label={memos.memoDisplay === "favorites" ? "Show active memos" : "Show favorite memos"}
					title={memos.memoDisplay === "favorites" ? "Active memos" : "Favorite memos"}
				>
					<Heart size={15} fill={memos.memoDisplay === "favorites" ? "currentColor" : "none"} />
				</button>
			{/if}
			{#if selected === "music" && musicPlayerVisible}
				<button class="music-list-action" type="button" onclick={() => void select(musicReturnView)} aria-label="Back to previous page" title="Back to previous page">
					<ArrowLeft size={15} />
				</button>
			{:else}
				{#if musicPlayerAvailable}
					<button
						class="music-list-action"
						type="button"
						onclick={openMusicPlayer}
						aria-label="Return to music player"
						title="Return to music player"
					>
						<Music2 size={15} />
					</button>
				{/if}
				<ScrollToTop />
			{/if}
		</div>
	</main>
</div>

{#if locked}
	<div class="lock-screen" role="dialog" aria-modal="true" aria-labelledby="lock-title">
		<div class="lock-card">
			<h1 id="lock-title">Locked</h1>
			<form onsubmit={unlockApp}>
				<label for="unlock-password">Password</label>
				<input bind:this={unlockInput} id="unlock-password" type="password" bind:value={unlockPassword} autocomplete="current-password" placeholder="Enter password" />
				{#if unlockError}<p class="unlock-error" role="alert">{unlockError}</p>{/if}
				<button type="submit" disabled={unlocking || unlockPassword === ""}>{unlocking ? "Unlocking…" : "Unlock"}</button>
			</form>
		</div>
	</div>
{/if}

<UpdateDialog {locked} onmodalchange={(open) => { updateModalOpen = open; }} />

<style>
	:global(html),
	:global(body),
	:global(#app) {
		width: 100%;
		height: 100%;
	}

	:global(body) {
		margin: 0;
		min-width: 320px;
		overflow: hidden;
		background: var(--color-background);
		color: var(--color-foreground);
		font-family: var(--font-sans);
	}

	:global(button) {
		font-family: inherit;
	}

	.shell {
		display: grid;
		grid-template-columns: var(--sidebar-width, 15rem) minmax(0, 1fr);
		height: 100vh;
		overflow: hidden;
	}

	.shell.locked { filter: blur(1rem); }

	.sidebar-overlay {
		display: none;
	}

	aside {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		height: 100vh;
		box-sizing: border-box;
		border-right: 1px solid var(--color-border);
		background: var(--color-background);
	}

	.sidebar-header { display: none; }

	.sidebar-resizer {
		position: absolute;
		top: 0;
		right: -0.2rem;
		bottom: 0;
		z-index: 4;
		width: 0.4rem;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: col-resize;
		touch-action: none;
	}

	.sidebar-resizer::after {
		position: absolute;
		top: 0;
		bottom: 0;
		left: calc(50% - 0.5px);
		width: 1px;
		background: var(--color-accent);
		content: "";
		opacity: 0;
	}

	.sidebar-resizer:hover::after,
	.sidebar-resizer:focus-visible::after { opacity: 1; }

	.close-sidebar {
		display: none;
		width: 1.75rem;
		height: 1.75rem;
		margin-left: auto;
		place-items: center;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
	}

	nav {
		display: grid;
		gap: 0.125rem;
		padding: 1.25rem 0.75rem 0;
	}

	nav button {
		display: flex;
		align-items: center;
		min-width: 0;
		height: 2.25rem;
		gap: 0.625rem;
		padding: 0 0.75rem;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 400;
		text-align: left;
	}

	nav button span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	nav button :global(svg) {
		flex-shrink: 0;
	}

	nav button:hover,
	nav button.active {
		border-color: transparent;
		background: color-mix(in srgb, var(--color-accent) 10%, transparent);
		color: var(--color-accent);
	}

	.sidebar-footer {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: auto;
		padding: 0.625rem 0.5rem;
		border-top: 1px solid var(--color-border);
	}

	.footer-controls {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
	}

	.footer-navigation,
	.footer-actions {
		display: flex;
		align-items: center;
		gap: 0.0625rem;
	}

	.footer-actions {
		padding-left: 0.0625rem;
	}

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

	.footer-controls button,
	.topbar button {
		position: relative;
		display: grid;
		width: 1.65rem;
		height: 1.65rem;
		place-items: center;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
	}

	.notification-dot {
		position: absolute;
		top: 0.25rem;
		right: 0.25rem;
		width: 0.375rem;
		height: 0.375rem;
		border: 1px solid var(--color-background);
		border-radius: var(--radius-full);
		background: var(--color-error);
	}

	.footer-controls button:hover,
	.topbar button:hover,
	.close-sidebar:hover {
		background: var(--color-muted);
		color: var(--color-foreground);
	}
	.footer-controls button.active { background: color-mix(in srgb, var(--color-accent) 10%, transparent); color: var(--color-accent); }

	main {
		min-width: 0;
		height: 100vh;
		overflow-y: auto;
		scrollbar-gutter: stable;
	}

	.topbar {
		position: sticky;
		top: 0;
		z-index: 10;
		display: none;
		height: 3rem;
		align-items: center;
		justify-content: flex-end;
		padding: 0 0.75rem;
		border-bottom: 1px solid var(--color-border);
		background: color-mix(in srgb, var(--color-background) 92%, transparent);
		backdrop-filter: blur(12px);
	}

	.topbar .menu-button,
	.topbar strong { display: none; }

	main.player-shell { display: flex; flex-direction: column; }
	.player-shell .topbar { flex-shrink: 0; }
	.player-shell .canvas { display: flex; flex: 1 0 auto; padding-bottom: 2rem; }
	.player-shell .page-content { display: flex; flex: 1; flex-direction: column; min-width: 0; }

	.global-scroll-action {
		position: fixed;
		right: 1.25rem;
		bottom: 1.25rem;
		z-index: 30;
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.memo-filter-action,
	.music-list-action {
		display: grid;
		width: 2.75rem;
		height: 2.75rem;
		padding: 0;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--color-border) 78%, transparent);
		border-radius: var(--radius-full);
		background: color-mix(in srgb, var(--color-background) 82%, transparent);
		box-shadow: var(--shadow-sm);
		color: var(--color-muted-foreground);
		cursor: pointer;
		backdrop-filter: blur(14px);
		transition:
			border-color var(--duration-fast),
			background var(--duration-fast),
			color var(--duration-fast),
			translate var(--duration-fast);
	}

	.memo-filter-action:hover,
	.music-list-action:hover {
		border-color: var(--color-border-strong);
		background: color-mix(in srgb, var(--color-background) 94%, var(--color-muted));
		color: var(--color-foreground);
		translate: 0 -2px;
	}

	.memo-filter-action.active {
		border-color: color-mix(in srgb, var(--color-accent) 55%, var(--color-border));
		background: color-mix(in srgb, var(--color-accent) 12%, var(--color-background));
		color: var(--color-accent);
	}

	.memo-filter-action:focus-visible,
	.music-list-action:focus-visible {
		outline: 2px solid var(--color-accent);
		outline-offset: 2px;
	}

	.tag-notice { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 0.75rem; margin-bottom: 1rem; padding: 0.75rem 1rem; border: 1px solid var(--color-error); border-radius: var(--radius-md); color: var(--color-error); font-size: 0.875rem; }
	.tag-notice button { border: 1px solid currentColor; border-radius: var(--radius-sm); padding: 0.25rem 0.5rem; background: transparent; color: inherit; font: inherit; cursor: pointer; }
	.tag-status { margin: 0 0 1rem; color: var(--color-muted-foreground); font-size: 0.875rem; }

	.consumer-error {
		width: 100%;
		margin: 0 auto;
	}

	.consumer-error header {
		margin-bottom: 1.5rem;
	}

	.consumer-error header p,
	.consumer-error header h1 {
		margin: 0;
	}

	.consumer-error header p {
		margin-bottom: 0.35rem;
		color: var(--color-accent);
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.error {
		display: flex;
		min-height: 12rem;
		align-items: center;
		justify-content: center;
		gap: 0.65rem;
		box-sizing: border-box;
		padding: 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-muted-foreground);
	}

	.error div {
		display: grid;
		gap: 0.2rem;
	}

	.error strong,
	.error span {
		font-size: 0.8rem;
	}

	.error button {
		height: 2rem;
		margin-left: 0.35rem;
		padding: 0 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-background);
		color: var(--color-foreground);
		cursor: pointer;
		font-size: 0.72rem;
	}

	.sentinel {
		height: 1px;
	}

	.loading-more {
		margin: 1.5rem 0 0;
		color: var(--color-muted-foreground);
		font-size: 0.75rem;
		text-align: center;
	}

	@media (min-width: 768px) {
		aside.compact nav {
			padding-right: 0.5rem;
			padding-left: 0.5rem;
		}

		aside.compact nav button {
			justify-content: center;
			padding: 0;
		}

		aside.compact nav button span {
			display: none;
		}

		aside.compact .footer-controls {
			display: none;
		}

		aside.compact .sidebar-footer {
			justify-content: center;
		}


	}

	@media (max-width: 767px) {
		.shell {
			grid-template-columns: 1fr;
		}

		aside {
			position: fixed;
			inset: 0 auto 0 0;
			z-index: 30;
			width: 15rem;
			translate: -100% 0;
			transition: translate var(--duration-fast);
		}

		aside.open {
			translate: 0 0;
		}

		.sidebar-overlay.open {
			position: fixed;
			inset: 0;
			z-index: 20;
			display: block;
			border: 0;
			background: var(--color-overlay);
		}

		.close-sidebar {
			display: grid;
		}

		.sidebar-resizer { display: none; }

		.sidebar-header {
			display: flex;
			justify-content: flex-end;
			padding: 0.75rem 0.75rem 0.25rem;
		}

		nav { padding-top: 0.25rem; }

		.topbar {
			display: flex;
			gap: 0.5rem;
			justify-content: flex-start;
		}

		.topbar .menu-button { display: grid; }

		.topbar strong {
			display: block;
			font-family: var(--font-serif);
			font-size: 0.95rem;
		}

	}
</style>
