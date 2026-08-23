<script lang="ts">
	import { Cloud, CloudFog, CloudLightning, CloudRain, CloudSun, Snowflake, Sun } from "@lucide/svelte";
	import { onMount } from "svelte";
	import type { Component } from "svelte";
	import type { Weather, WeatherReport } from "../consumer";

	let { weather, error }: { weather: WeatherReport | null; error: string | null } = $props();
	let now = $state(new Date());

	function condition(code: number): string {
		if (code === 0) return "晴";
		if (code <= 3) return "多云";
		if (code === 45 || code === 48) return "雾";
		if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82)) return "雨";
		if ((code >= 71 && code <= 77) || (code >= 85 && code <= 86)) return "雪";
		if (code >= 95) return "雷雨";
		return "天气";
	}

	function weatherIcon(code: number): Component {
		if (code === 0) return Sun;
		if (code <= 3) return CloudSun;
		if (code === 45 || code === 48) return CloudFog;
		if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82)) return CloudRain;
		if ((code >= 71 && code <= 77) || (code >= 85 && code <= 86)) return Snowflake;
		if (code >= 95) return CloudLightning;
		return Cloud;
	}

	function localTime(timezone: string): string {
		return new Intl.DateTimeFormat("zh-CN", {
			timeZone: timezone,
			hour: "2-digit",
			minute: "2-digit",
			second: "2-digit",
			hourCycle: "h23",
		}).format(now);
	}

	function localDate(timezone: string): string {
		return new Intl.DateTimeFormat("zh-CN", {
			timeZone: timezone,
			weekday: "short",
			month: "numeric",
			day: "numeric",
		}).format(now);
	}

	function forecastHour(value: string): string {
		const time = value.split("T")[1];
		return time?.slice(0, 5) ?? value;
	}

	onMount(() => {
		const timer = window.setInterval(() => {
			now = new Date();
		}, 1_000);
		return () => window.clearInterval(timer);
	});
</script>

{#snippet city(item: Weather)}
	<article>
		<div class="heading">
			<span><CloudSun size={15} /> {item.location}</span>
			<small>{item.timezoneAbbreviation}</small>
		</div>
		<div class="local-context">
			<div>
				<strong>{localTime(item.timezone)}</strong>
				<small>{localDate(item.timezone)}</small>
			</div>
			<span>未来 6 小时</span>
		</div>
		<div class="forecast" aria-label={`${item.location} future six hour forecast`}>
			{#each item.forecast as hour (hour.time)}
				{@const Icon = weatherIcon(hour.weatherCode)}
				<div title={condition(hour.weatherCode)}>
					<time datetime={hour.time}>{forecastHour(hour.time)}</time>
					<Icon size={15} aria-label={condition(hour.weatherCode)} />
					<strong>{Math.round(hour.temperature2m)}°</strong>
				</div>
			{/each}
		</div>
	</article>
{/snippet}

<section class="context-panel" aria-label="Weather and world clocks">
	{#if weather !== null}
		{@render city(weather.shanghai)}
		{@render city(weather.ningbo)}
		{@render city(weather.nottingham)}
	{:else if error !== null}
		<div class="weather-message">{error}</div>
	{:else}
		<div class="weather-message">Reading future conditions…</div>
	{/if}
</section>

<style>
	.context-panel { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.75rem; margin-top: 0.75rem; }
	article,
	.weather-message { min-width: 0; padding: 0.9rem 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); box-shadow: var(--shadow-xs); }
	article { transition: transform var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), border-color var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1), box-shadow var(--duration-slow) cubic-bezier(0.16, 1, 0.3, 1); }
	article:hover { transform: translateY(-2px); border-color: var(--color-accent); box-shadow: var(--shadow-sm); }
	.heading,
	.heading span,
	.local-context { display: flex; align-items: center; }
	.heading { justify-content: space-between; gap: 0.75rem; color: var(--color-muted-foreground); }
	.heading span { gap: 0.4rem; font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
	.local-context { justify-content: space-between; gap: 0.75rem; margin-top: 0.75rem; }
	.local-context > div { display: grid; gap: 0.08rem; }
	.local-context strong { font-family: var(--font-mono); font-size: 1.15rem; font-weight: 500; letter-spacing: -0.03em; }
	.local-context > span { color: var(--color-muted-foreground); font-size: 0.55rem; }
	.forecast { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 0.15rem; margin-top: 0.8rem; padding-top: 0.65rem; border-top: 1px solid var(--color-divider); }
	.forecast > div { display: grid; min-width: 0; justify-items: center; gap: 0.3rem; color: var(--color-muted-foreground); }
	.forecast time { font-family: var(--font-mono); font-size: 0.48rem; }
	.forecast strong { color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.62rem; font-weight: 500; }
	.weather-message { grid-column: 1 / -1; color: var(--color-muted-foreground); font-size: 0.68rem; }
	small { margin: 0; color: var(--color-muted-foreground); font-size: 0.56rem; }
	@media (max-width: 760px) { .context-panel { grid-template-columns: 1fr; } }
	@media (prefers-reduced-motion: reduce) { article { transition: none; } article:hover { transform: none; } }
</style>
