<script lang="ts">
	import { Bell } from "@lucide/svelte";
</script>

<section class="inbox" aria-label="Inbox">
	<header><h1>Inbox</h1><p>Notifications will appear here.</p></header>
	<div class="empty">
		<span><Bell size={16} /></span>
		<strong>No notifications</strong>
		<p>Knowledge and Newspaper updates are not sent to this inbox.</p>
	</div>
</section>

<style>
	.inbox { width: min(100%, 45rem); margin: 0 auto; }
	.inbox header { margin-bottom: 1.5rem; }
	.inbox h1 { margin: 0; font-family: var(--font-serif); font-size: 1.75rem; font-weight: 500; }
	.inbox header p { margin: 0.5rem 0 0; color: var(--color-muted-foreground); font-size: 0.8rem; }
	.empty { display: grid; justify-items: center; gap: 0.5rem; padding: 3rem 0; border-top: 1px solid var(--color-border); text-align: center; }
	.empty span { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: var(--radius-full); background: var(--color-muted); color: var(--color-muted-foreground); }
	.empty strong { font-size: 0.85rem; font-weight: 500; }
	.empty p { margin: 0; color: var(--color-muted-foreground); font-size: 0.75rem; }
</style>
