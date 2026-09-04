<script lang="ts">
	import { openUrl } from "@tauri-apps/plugin-opener";
	import { Bell, BadgeCheck, GitCommitHorizontal, GitFork, GitPullRequest, MessageSquareText } from "@lucide/svelte";
	import type { GithubActivity, GithubSnapshot } from "../consumer";

	let { github, error }: { github: GithubSnapshot | null; error: string | null } = $props();

	let calendarScroll = $state<HTMLDivElement | null>(null);

	$effect(() => {
		if (github === null || calendarScroll === null) return;
		const element = calendarScroll;
		element.scrollLeft = element.scrollWidth;
		const observer = new ResizeObserver(() => { element.scrollLeft = element.scrollWidth; });
		observer.observe(element);
		return () => observer.disconnect();
	});

	const dateFormatter = new Intl.DateTimeFormat("en", { day: "numeric", month: "short" });

	function calendarRow(date: string) {
		return new Date(`${date}T00:00:00Z`).getUTCDay() + 1;
	}

	function activityLabel(activity: GithubActivity) {
		switch (activity.kind) {
			case "approve": return "Approved";
			case "pullRequest": return "Pull request";
			case "review": return "Reviewed";
			default: return "Committed";
		}
	}

	function activityDate(date: string) {
		return dateFormatter.format(new Date(date));
	}
</script>

<section class="github-panel" aria-label="GitHub contributions, activity and notifications">
	{#if error !== null && github === null}
		<div class="github-message" role="alert">
			<GitFork size={17} />
			<div><strong>GitHub unavailable</strong><span>{error}</span></div>
		</div>
	{:else if github === null}
		<div class="github-message">
			<GitFork size={17} />
			<span>Loading GitHub activity…</span>
		</div>
	{:else}
		<div class="contributions">
			<header>
				<div><GitFork size={15} /><strong>Contributions</strong></div>
				<button type="button" onclick={() => void openUrl(github.profileUrl)}>@{github.login}</button>
			</header>
			<p><strong>{github.totalContributions}</strong> contributions in the last year</p>
			<div class="calendar-scroll" bind:this={calendarScroll}>
				<div class="calendar" role="img" aria-label={`${github.totalContributions} GitHub contributions in the last year`}>
					{#each github.weeks as week}
						<div class="week">
							{#each week.days as day}
								<span
									class={`level-${day.level}`}
									style:grid-row={calendarRow(day.date)}
									title={`${day.date}: ${day.count} contribution${day.count === 1 ? "" : "s"}`}
								></span>
							{/each}
						</div>
					{/each}
				</div>
			</div>
			<div class="legend"><span>Less</span>{#each [0, 1, 2, 3, 4] as level}<i class={`level-${level}`}></i>{/each}<span>More</span></div>
		</div>

		<div class="recent">
			<header><strong>Recent activity</strong><span>Latest 3</span></header>
			{#if github.recentActivity.length === 0}
				<p class="empty">No recent contribution activity</p>
			{:else}
				<div class="activity-list">
					{#each github.recentActivity as activity}
						<button type="button" class="activity" onclick={() => void openUrl(activity.url)}>
							<span class="activity-icon">
								{#if activity.kind === "approve"}<BadgeCheck size={14} />
								{:else if activity.kind === "pullRequest"}<GitPullRequest size={14} />
								{:else if activity.kind === "review"}<MessageSquareText size={14} />
								{:else}<GitCommitHorizontal size={14} />{/if}
							</span>
							<span class="activity-copy">
								<span><strong>{activityLabel(activity)}</strong><time datetime={activity.occurredAt}>{activityDate(activity.occurredAt)}</time></span>
								<b>{activity.title}</b>
								<small>{activity.repository}</small>
							</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
		<div class="notifications">
			<header><strong>Notifications</strong><button type="button" onclick={() => void openUrl("https://github.com/notifications")}>Open inbox ↗</button></header>
			{#if github.notifications.status === "failed"}
				<p class="notification-error" role="alert">{github.notifications.message}</p>
			{:else if github.notifications.items.length === 0}
				<p class="empty">No unread notifications</p>
			{:else}
				<p class="notification-count">{github.notifications.items.length}{github.notifications.hasMore ? "+" : ""} unread</p>
				<div class="notification-list">
					{#each github.notifications.items as notification (notification.id)}
						<button type="button" class="activity" disabled={notification.url === null} title={notification.url === null ? "View this notification in GitHub Inbox" : notification.title} onclick={() => { if (notification.url !== null) void openUrl(notification.url); }}>
							<span class="activity-icon">{#if notification.reason === "review_requested"}<GitPullRequest size={14} />{:else}<Bell size={14} />{/if}</span>
							<span class="activity-copy">
								<span><strong>{notification.reason === "review_requested" ? "Review requested" : notification.reason.replaceAll("_", " ")}</strong><time datetime={notification.updatedAt}>{activityDate(notification.updatedAt)}</time></span>
								<b>{notification.title}</b><small>{notification.repository}</small>
							</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</section>

<style>
	.github-panel {
		display: grid;
		grid-template-columns: minmax(0, 1.3fr) minmax(13rem, 1fr) minmax(15rem, 1fr);
		min-height: 11rem;
		margin-top: 0.75rem;
		overflow: hidden;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-xs);
	}

	.github-message { display: flex; grid-column: 1 / -1; align-items: center; justify-content: center; gap: 0.55rem; padding: 1rem; color: var(--color-muted-foreground); font-size: 0.75rem; }
	.github-message div { display: flex; flex-direction: column; gap: 0.2rem; }
	.github-message strong { color: var(--color-foreground); }
	.contributions,
	.recent,
	.notifications { min-width: 0; padding: 1rem; }
	.recent, .notifications { border-left: 1px solid var(--color-divider); }
	header { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
	header > div { display: flex; align-items: center; gap: 0.4rem; }
	header strong { font-size: 0.72rem; font-weight: 600; text-transform: uppercase; }
	header > span,
	header button { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.58rem; }
	header button { padding: 0; border: 0; background: transparent; cursor: pointer; }
	header button:hover { color: var(--color-success); }
	.contributions > p { margin: 0.65rem 0 0.85rem; color: var(--color-muted-foreground); font-size: 0.65rem; }
	.contributions > p strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.8rem; }
	.calendar-scroll { overflow-x: auto; scrollbar-width: none; }
	.calendar-scroll::-webkit-scrollbar { display: none; }
	.calendar { display: flex; width: max-content; gap: 3px; padding-bottom: 0.3rem; }
	.week { display: grid; grid-template-rows: repeat(7, 9px); gap: 3px; }
	.week span,
	.legend i { width: 9px; height: 9px; border-radius: var(--radius-xs); background: var(--color-muted); }
	.level-1 { background: color-mix(in srgb, var(--color-success) 30%, var(--color-muted)) !important; }
	.level-2 { background: color-mix(in srgb, var(--color-success) 52%, var(--color-muted)) !important; }
	.level-3 { background: color-mix(in srgb, var(--color-success) 76%, var(--color-muted)) !important; }
	.level-4 { background: var(--color-success) !important; }
	.legend { display: flex; align-items: center; justify-content: flex-end; gap: 3px; margin-top: 0.55rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.48rem; }
	.legend span:first-child { margin-right: 0.2rem; }
	.legend span:last-child { margin-left: 0.2rem; }
	.legend i { display: inline-block; }
	.notification-list { display: flex; flex-direction: column; max-height: 10.5rem; overflow-y: auto; }
	.notification-count { margin: 0.5rem 0 0; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.55rem; }
	.notification-error { color: var(--color-muted-foreground); font-size: 0.65rem; line-height: 1.5; overflow-wrap: anywhere; }
	.activity:disabled { cursor: default; }
	.activity-list { display: flex; flex-direction: column; margin-top: 0.65rem; }
	.activity { display: grid; grid-template-columns: 1.65rem minmax(0, 1fr); gap: 0.45rem; padding: 0.55rem 0; border: 0; border-top: 1px solid var(--color-divider); background: transparent; color: inherit; cursor: pointer; text-align: left; }
	.activity:first-child { border-top: 0; }
	.activity-icon { display: inline-flex; width: 1.65rem; height: 1.65rem; align-items: center; justify-content: center; border-radius: var(--radius-md); background: color-mix(in srgb, var(--color-success) 12%, var(--color-muted)); color: var(--color-success); }
	.activity-copy { display: flex; min-width: 0; flex-direction: column; gap: 0.15rem; }
	.activity-copy > span { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
	.activity-copy strong { color: var(--color-success); font-size: 0.55rem; font-weight: 600; text-transform: uppercase; }
	.activity-copy time,
	.activity-copy small { color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.5rem; }
	.activity-copy b { overflow: hidden; color: var(--color-foreground); font-size: 0.68rem; font-weight: 550; text-overflow: ellipsis; white-space: nowrap; }
	.activity:hover .activity-copy b { color: var(--color-success); }
	.empty { display: flex; min-height: 6rem; align-items: center; justify-content: center; margin: 0; color: var(--color-muted-foreground); font-size: 0.65rem; }

	@media (max-width: 760px) {
		.github-panel { grid-template-columns: 1fr; }
		.recent, .notifications { border-top: 1px solid var(--color-divider); border-left: 0; }
	}
</style>
