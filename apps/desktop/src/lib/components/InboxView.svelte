<script lang="ts">
	import { Bell, Check } from "@lucide/svelte";
	import type { CommandResponse, NtfyNotification } from "../consumer";

	let {
		notifications,
		onread,
	}: {
		notifications: NtfyNotification[];
		onread: (id: string) => Promise<CommandResponse<NtfyNotification[]>>;
	} = $props();
	let markingRead = $state<string | null>(null);
	let error = $state<string | null>(null);
	const timestamp = new Intl.DateTimeFormat("zh-CN", {
		dateStyle: "medium",
		timeStyle: "short",
	});

	async function markRead(id: string) {
		if (markingRead !== null) return;
		markingRead = id;
		error = null;
		const response = await onread(id);
		markingRead = null;
		if (response.status === "failed") error = response.message;
	}
</script>

<section class="inbox" aria-label="Inbox">
	<header><h1>Inbox</h1><p>Notifications will appear here.</p></header>
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#if notifications.length === 0}
		<div class="empty">
			<span><Bell size={16} /></span>
			<strong>No notifications</strong>
			<p>Notifications will appear after the ntfy subscription is configured.</p>
		</div>
	{:else}
		<div class="notifications">
			{#each notifications as notification (notification.id)}
				<article>
					<header><span>{notification.source}</span><time datetime={new Date(notification.timestamp * 1_000).toISOString()}>{timestamp.format(new Date(notification.timestamp * 1_000))}</time></header>
					{#if notification.title}<h2>{notification.title}</h2>{/if}
					<p>{notification.message}</p>
					<button type="button" disabled={markingRead !== null} onclick={() => void markRead(notification.id)}><Check size={12} />{markingRead === notification.id ? "Marking…" : "Mark as read"}</button>
				</article>
			{/each}
		</div>
	{/if}
</section>

<style>
	.inbox { width: min(100%, 45rem); margin: 0 auto; }
	.inbox > header { margin-bottom: 1.5rem; }
	.inbox h1 { margin: 0; font-family: var(--font-serif); font-size: 1.75rem; font-weight: 500; }
	.inbox > header p { margin: 0.5rem 0 0; color: var(--color-muted-foreground); font-size: 0.8rem; }
	.error { margin: 0 0 1rem; color: var(--color-error); font-size: 0.75rem; }
	.empty { display: grid; justify-items: center; gap: 0.5rem; padding: 3rem 0; border-top: 1px solid var(--color-border); text-align: center; }
	.empty span { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-muted-foreground); }
	.empty strong { font-size: 0.85rem; font-weight: 500; }
	.empty p { margin: 0; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.notifications { border-top: 1px solid var(--color-border); }
	article { padding: 1rem 0; }
	article + article { border-top: 1px solid var(--color-border); }
	article header { display: flex; justify-content: space-between; gap: 1rem; margin: 0 0 0.5rem; color: var(--color-muted-foreground); font-size: 0.68rem; }
	article header span { color: var(--color-foreground); font-weight: 500; }
	article h2 { margin: 0 0 0.4rem; font-size: 0.9rem; font-weight: 500; }
	article p { margin: 0; color: var(--color-muted-foreground); font-size: 0.78rem; line-height: 1.6; white-space: pre-wrap; }
	article > button { display: flex; align-items: center; gap: 0.35rem; margin: 0.75rem 0 0 auto; padding: 0.3rem 0.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font-size: 0.7rem; }
	article > button:hover { background: var(--color-muted); color: var(--color-foreground); }
	article > button:disabled { cursor: wait; opacity: 0.6; }
</style>
