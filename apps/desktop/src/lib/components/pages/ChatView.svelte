<script lang="ts">
	import { ArrowUp, Plus, Square } from "@lucide/svelte";
	import { Button, Textarea } from "@my-workspace/ui";
	import ConnectionStatus from "../layout/ConnectionStatus.svelte";
	import { tick } from "svelte";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import type { ChatSnapshot } from "@/lib/contracts/chat";
	import type { createChatSession } from "../chat/session.svelte";
	import "../knowledge/prose.css";

	let { session, profileAvatar }: { session: ReturnType<typeof createChatSession>; profileAvatar: string } = $props();
	let thread = $state<HTMLDivElement | null>(null);
	let atBottom = $state(true);
	let interactionError = $state<string | null>(null);
	const snapshot = $derived(session.snapshot);
	const turns = $derived.by(() => {
		const turns: [ChatSnapshot["messages"][number], ...ChatSnapshot["messages"]][] = [];
		for (const message of snapshot.messages) {
			const previous = turns.at(-1);
			if (previous !== undefined && message.role === "assistant" && previous[0].role === "assistant") {
				previous.push(message);
			} else {
				turns.push([message]);
			}
		}
		return turns;
	});

	$effect(() => {
		void snapshot.revision;
		if (atBottom) void tick().then(() => { if (thread !== null) thread.scrollTop = thread.scrollHeight; });
	});

	function externalLinks(node: HTMLDivElement) {
		function openLink(event: MouseEvent) {
			if (!(event.target instanceof Element)) return;
			const link = event.target.closest("a");
			if (link === null) return;
			event.preventDefault();
			if (/^https?:\/\//i.test(link.href)) void openUrl(link.href).then(() => {}, () => { interactionError = "Could not open the link."; });
		}
		node.addEventListener("click", openLink);
		return { destroy() { node.removeEventListener("click", openLink); } };
	}

</script>

<section class="chat" aria-label="Chat">
	<header class="page-header">
		<div><h1>Chat</h1><p class="page-description">{snapshot.connected ? snapshot.model : "Talk with Pi."}</p></div>
		<div class="actions">
			{#if snapshot.connected}<Button variant="ghost" size="icon-sm" disabled={snapshot.busy || session.pending} aria-label="New chat" onclick={() => void session.control("newSession")}><Plus size={14} /></Button>{/if}
			<ConnectionStatus name="Pi" connected={snapshot.connected} connecting={session.connecting} ontoggle={() => { if (snapshot.connected) void session.control("disconnect"); else void session.connect(); }} />
		</div>
	</header>

	<div class="thread" bind:this={thread} onscroll={() => { if (thread !== null) atBottom = thread.scrollHeight - thread.scrollTop - thread.clientHeight < 40; }}>
		{#if snapshot.messages.length === 0}
			<div class="empty"><p>喵？今天过得怎么样</p></div>
		{:else}
			{#each turns as turn (turn[0].id)}
				{@const role = turn[0].role}
				<div class="message" class:user={role === "user"}>
					<img class="avatar" src={profileAvatar} alt={role === "user" ? "You" : "Pi"} />
					<div class="body" class:bubble={role === "user"}>
						{#each turn as message (message.id)}
							{#each message.parts as part, index (index)}
								{#if part.kind === "text"}
									{#if message.role === "assistant" && part.html}
										<div class="knowledge-prose" data-content-typography use:externalLinks>{@html part.html}</div>
									{:else}<span class="text">{part.text}</span>{/if}
								{:else if part.kind === "thinking" && part.text}
									<details class="tool"><summary>Thinking</summary><pre>{part.text}</pre></details>
								{:else if part.kind === "tool"}
									<details class="tool"><summary><code>{part.name}</code><span>{part.state}</span></summary><div><small>Arguments</small><pre>{part.arguments}</pre>{#if part.output}<small>Result</small><pre>{part.output}</pre>{/if}</div></details>
								{/if}
							{/each}
							{#if snapshot.busy && message.role === "assistant" && message === snapshot.messages.at(-1) && message.parts.length === 0}<span class="thinking" role="status">Thinking…</span>{/if}
						{/each}

					</div>
				</div>
			{/each}
		{/if}
	</div>

	{#if session.error || snapshot.error || interactionError}<p class="error" role="alert">{session.error || snapshot.error || interactionError}</p>{/if}
	<form class="composer" onsubmit={(event) => { event.preventDefault(); void session.send(); }}>
		<Textarea class="min-h-0 resize-none" bind:value={session.draft} aria-label="Message Pi" placeholder="Message…" rows={2} disabled={!snapshot.connected} onkeydown={(event: KeyboardEvent) => { if (event.key === "Enter" && !event.shiftKey && !event.isComposing) { event.preventDefault(); void session.send(); } }} />
		{#if snapshot.busy}<Button variant="secondary" size="icon-sm" aria-label="Stop response" onclick={() => void session.control("stop")}><Square size={14} /></Button>
		{:else}<Button type="submit" size="icon-sm" aria-label="Send message" disabled={!snapshot.connected || session.pending || session.draft.trim() === ""}><ArrowUp size={16} /></Button>{/if}
	</form>
</section>

<style>
	.chat { display: flex; flex: 1; flex-direction: column; width: 100%; min-height: 0; }
	.page-header, .composer, .error { flex-shrink: 0; }
	.actions { display: flex; align-items: center; gap: 0.25rem; }
	.thread { display: flex; flex: 1; flex-direction: column; gap: 0.25rem; min-height: 0; overflow-y: auto; padding-block: 1rem; scrollbar-width: thin; }
	.empty { display: grid; flex: 1; place-items: center; color: var(--color-muted-foreground); font-family: var(--font-serif); font-size: 1.125rem; font-style: italic; }
	.message { display: flex; align-items: flex-start; gap: 0.625rem; padding: 0.5rem 0.25rem; }
	.message.user { flex-direction: row-reverse; }
	.avatar { display: grid; flex-shrink: 0; width: 1.75rem; height: 1.75rem; object-fit: cover; border: 1px solid var(--color-border); border-radius: var(--radius-full); color: var(--color-muted-foreground); }
	.body :global(.knowledge-prose) { margin-top: 0; font-size: inherit; }
	.body { display: flex; flex: 1; flex-direction: column; align-items: flex-start; gap: 0.5rem; min-width: 0; font-size: 0.875rem; line-height: 1.65; overflow-wrap: anywhere; }
	.bubble { flex: 0 1 auto; max-width: 72%; padding: 0.5625rem 0.8125rem; border-radius: var(--radius-xl); border-bottom-right-radius: var(--radius-xs); background: var(--color-accent); color: var(--color-accent-foreground); }
	.text { white-space: pre-wrap; }
	.tool { width: 100%; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.75rem; }
	.tool summary { cursor: pointer; }
	.tool summary span { margin-left: 0.5rem; }
	.tool pre { max-height: 16rem; overflow: auto; padding: 0.5rem 0.625rem; border-radius: var(--radius-md); background: var(--color-muted); color: var(--color-foreground); font-size: 0.75rem; white-space: pre-wrap; overflow-wrap: anywhere; }
	.tool small { font-family: var(--font-sans); }
	.thinking { color: var(--color-muted-foreground); }
	.composer { display: flex; align-items: flex-end; gap: 0.5rem; margin-top: 1rem; }
	.composer :global([data-slot="textarea"]) { flex: 1; min-width: 0; max-height: 9rem; }
	.error { margin: 0.5rem 0; color: var(--color-error); font-size: 0.8125rem; }
</style>
