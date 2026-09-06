<script lang="ts">
	import { ArrowUp } from "@lucide/svelte";
	import { onMount } from "svelte";

	const radius = 11;
	const circumference = 2 * Math.PI * radius;
	let marker = $state<HTMLSpanElement | null>(null);
	let scroller: HTMLElement | null = null;
	let progress = $state(0);

	function update() {
		if (scroller === null) return;
		const scrollable = scroller.scrollHeight - scroller.clientHeight;
		progress = scrollable <= 0 ? 0 : Math.min(1, scroller.scrollTop / scrollable);
	}

	function scrollToTop() {
		if (scroller === null) return;
		const behavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth";
		scroller.scrollTo({ top: 0, behavior });
	}

	onMount(() => {
		if (marker === null) return;
		const main = marker.closest("main");
		if (main === null) return;
		scroller = main;
		update();
		main.addEventListener("scroll", update, { passive: true });
		window.addEventListener("resize", update);
		return () => {
			main.removeEventListener("scroll", update);
			window.removeEventListener("resize", update);
		};
	});
</script>

<span bind:this={marker} class="marker"></span>
<button type="button" onclick={scrollToTop} aria-label="Back to top" title="Back to top">
	<svg aria-hidden="true" viewBox="0 0 28 28">
		<circle class="track" cx="14" cy="14" fill="none" r={radius} stroke-width="1.25" />
		<circle
			class="progress"
			cx="14"
			cy="14"
			fill="none"
			r={radius}
			stroke-dasharray={circumference}
			stroke-dashoffset={circumference * (1 - progress)}
			stroke-linecap="round"
			stroke-width="1.5"
		/>
	</svg>
	<ArrowUp size={14} strokeWidth={1.8} />
</button>

<style>
	.marker { display: none; }
	button {
		position: relative;
		display: grid;
		width: 2.75rem;
		height: 2.75rem;
		padding: 0;
		place-items: center;
		border: 0;
		border-radius: var(--radius-full);
		background: color-mix(in srgb, var(--color-background) 82%, transparent);
		box-shadow: var(--shadow-sm);
		color: var(--color-muted-foreground);
		cursor: pointer;
		backdrop-filter: blur(14px);
		animation: appear var(--duration-fast) ease-out;
		transition:
			border-color var(--duration-fast),
			background var(--duration-fast),
			color var(--duration-fast),
			translate var(--duration-fast);
	}
	button:hover {
		border-color: var(--color-border-strong);
		background: color-mix(in srgb, var(--color-background) 94%, var(--color-muted));
		color: var(--color-foreground);
		translate: 0 -2px;
	}
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	svg { position: absolute; inset: 0; width: 100%; height: 100%; rotate: -90deg; }
	.track { stroke: color-mix(in srgb, var(--color-border) 72%, transparent); }
	.progress { stroke: var(--color-accent); }
	@keyframes appear { from { opacity: 0; scale: 0.85; } }
	@media (prefers-reduced-motion: reduce) { button { animation: none; } }
</style>
