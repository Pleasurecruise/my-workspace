<script lang="ts">
	import { Activity, CircleAlert, CircleCheck, Wrench } from "@lucide/svelte";
	import type { ServiceStatusCatalogEntry, ServiceStatusLevel, ServiceStatusReport } from "../consumer";

	let { report, catalog, serviceId, error }: { report: ServiceStatusReport | null; catalog: ServiceStatusCatalogEntry[]; serviceId: string; error: string | null } = $props();
	let service = $derived.by(() => {
		if (report === null) return null;
		for (const item of report.services) {
			if (item.serviceId === serviceId) return item;
		}
		return null;
	});
	let failure = $derived.by(() => {
		if (report === null) return null;
		for (const item of report.failures) {
			if (item.serviceId === serviceId) return item;
		}
		return null;
	});
	let name = $derived.by(() => {
		for (const item of catalog) {
			if (item.id === serviceId) return item.name;
		}
		if (service !== null) return service.name;
		return serviceId;
	});

	function label(status: ServiceStatusLevel): string {
		switch (status) {
			case "operational": return "Operational";
			case "underMaintenance": return "Under maintenance";
			case "degradedPerformance": return "Degraded performance";
			case "partialOutage": return "Partial outage";
			case "majorOutage": return "Major outage";
			case "unknown": return "Status unknown";
		}
	}

	function time(value: string): string {
		const timestamp = new Date(value);
		if (Number.isNaN(timestamp.getTime())) return "Update time unavailable";
		return new Intl.DateTimeFormat("en-US", { hour: "2-digit", minute: "2-digit" }).format(timestamp);
	}
</script>

<article class="service-status-panel" aria-label={`${name} service status`}>
	<header>
		<span><Activity size={15} /> {name}</span>
		<small>STATUS</small>
	</header>
	{#if service !== null}
		<div class="summary">
			<div class:operational={service.status === "operational"} class:maintenance={service.status === "underMaintenance"} class:unavailable={service.status === "partialOutage" || service.status === "majorOutage"} class="status-icon">
				{#if service.status === "operational"}<CircleCheck size={19} />{:else if service.status === "underMaintenance"}<Wrench size={18} />{:else}<CircleAlert size={19} />{/if}
			</div>
			<div><strong>{label(service.status)}</strong><span>{service.operationalComponents}/{service.totalComponents} components operational</span></div>
			<time datetime={service.updatedAt}>{time(service.updatedAt)}</time>
		</div>
		<div
			class:degraded={service.status !== "operational" && service.status !== "underMaintenance"}
			class:maintenance={service.status === "underMaintenance"}
			class="health-bar"
			role="progressbar"
			aria-label={`${service.name} operational components`}
			aria-valuenow={service.operationalPercent}
			aria-valuemin="0"
			aria-valuemax="100"
		><span style:width={`${service.operationalPercent}%`}></span></div>
		<div class="labels"><span>Current health</span><strong>{Math.round(service.operationalPercent)}%</strong></div>
		{#if service.activeIncidents > 0}<p>{service.activeIncidents} active {service.activeIncidents === 1 ? "incident" : "incidents"}</p>{/if}
	{:else if error !== null}
		<p class="message" role="alert">{error}</p>
	{:else if failure !== null}
		<p class="message" role="alert">{failure.message}</p>
	{:else}
		<p class="message">Loading service status…</p>
	{/if}
</article>

<style>
	.service-status-panel { width: 100%; min-width: 0; box-sizing: border-box; padding: 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); transition: transform var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), border-color var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), box-shadow var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1); }
	.service-status-panel:hover { transform: translateY(-2px); border-color: var(--color-accent); box-shadow: var(--shadow-sm); }
	header,
	header span,
	.summary,
	.labels { display: flex; align-items: center; }
	header { justify-content: space-between; color: var(--color-muted-foreground); }
	header span { gap: 0.4rem; font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
	header small { color: var(--color-accent); font-family: var(--font-mono); font-size: 0.5rem; letter-spacing: 0.08em; }
	.summary { gap: 0.55rem; margin-top: 0.85rem; }
	.summary > div:nth-child(2) { display: grid; min-width: 0; gap: 0.08rem; }
	.summary strong { font-size: 0.76rem; font-weight: 600; }
	.summary span,
	.summary time { color: var(--color-muted-foreground); font-size: 0.52rem; }
	.summary time { margin-left: auto; font-family: var(--font-mono); white-space: nowrap; }
	.status-icon { display: grid; width: 2rem; height: 2rem; flex: 0 0 auto; place-items: center; border-radius: var(--radius-full); background: color-mix(in srgb, var(--color-warning) 12%, transparent); color: var(--color-warning); }
	.status-icon.operational { background: color-mix(in srgb, var(--color-success) 12%, transparent); color: var(--color-success); }
	.status-icon.maintenance { background: color-mix(in srgb, var(--color-accent) 12%, transparent); color: var(--color-accent); }
	.status-icon.unavailable { background: color-mix(in srgb, var(--color-error) 12%, transparent); color: var(--color-error); }
	.health-bar { height: 0.55rem; margin-top: 1rem; overflow: hidden; border-radius: var(--radius-full); background: var(--color-muted); }
	.health-bar span { display: block; height: 100%; border-radius: inherit; background: var(--color-success); transition: width var(--duration-progress) cubic-bezier(0.16, 1, 0.3, 1); }
	.health-bar.degraded span { background: var(--color-error); }
	.health-bar.maintenance span { background: var(--color-accent); }
	.labels { justify-content: space-between; margin-top: 0.4rem; color: var(--color-muted-foreground); font-size: 0.5rem; }
	.labels strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.55rem; }
	p { margin: 0.5rem 0 0; color: var(--color-warning); font-size: 0.55rem; }
	p.message { min-height: 4.7rem; display: grid; place-items: center; color: var(--color-muted-foreground); font-size: 0.68rem; text-align: center; }
	@media (prefers-reduced-motion: reduce) { .service-status-panel, .health-bar span { transition: none; } .service-status-panel:hover { transform: none; } }
</style>
