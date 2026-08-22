<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import Button from "@my-workspace/ui/components/button";
	import { appName, foundations } from "./lib/app";
	import { initTheme } from "./lib/theme";

	let message = $state<string>();
	let error = $state<string>();
	let pending = $state(false);

	initTheme();

	async function loadHello() {
		if (pending) return;
		pending = true;
		error = undefined;
		try {
			message = await invoke<string>("hello");
		} catch (cause) {
			error = cause instanceof Error ? cause.message : String(cause);
		} finally {
			pending = false;
		}
	}

	onMount(() => {
		void loadHello();
	});
</script>

<svelte:head>
	<title>{appName}</title>
	<meta
		name="description"
		content="A Rust-first desktop application foundation with a Svelte interface."
	/>
</svelte:head>

<main>
	<section class="hero" aria-labelledby="page-title">
		<p class="eyebrow">Local-first publishing control plane</p>
		<h1 id="page-title">{appName}</h1>
		<p class="lede">
			Create and manage content locally, then distribute it through explicit adapters to the
			projects that publish it.
		</p>

		<div class="foundations" aria-label="Technology boundaries">
			{#each foundations as foundation}
				<div class="foundation">
					<span>{foundation.label}</span>
					<strong>{foundation.value}</strong>
				</div>
			{/each}
		</div>

		<section class="bridge" aria-labelledby="hello-title">
			<div class="section-heading">
				<div>
					<p class="section-label">Rust CMS core</p>
					<h2 id="hello-title">Hello World</h2>
				</div>
				<Button type="button" variant="outline" disabled={pending} onclick={() => void loadHello()}>
					{pending ? "Loading…" : "Refresh"}
				</Button>
			</div>

			{#if error}
				<p class="error" role="alert">{error}</p>
			{:else if message}
				<output aria-live="polite">{message}</output>
			{:else}
				<p class="loading" aria-live="polite">Loading the local workspace…</p>
			{/if}
		</section>
	</section>
</main>

<style>
	:global(body) {
		margin: 0;
		min-width: 320px;
		min-height: 100vh;
		background: var(--color-background);
		color: var(--color-foreground);
		font-family: var(--font-sans);
	}

	main {
		display: grid;
		min-height: 100vh;
		place-items: center;
		padding: 2rem;
	}

	.hero {
		width: min(100%, 46rem);
		padding: clamp(2rem, 6vw, 4.5rem);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		background: var(--color-muted);
		box-shadow: var(--shadow-sm);
	}

	.eyebrow {
		margin: 0 0 1rem;
		color: var(--color-accent);
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.14em;
		text-transform: uppercase;
	}

	h1 {
		margin: 0;
		font-family: var(--font-serif);
		font-size: clamp(3rem, 9vw, 6rem);
		font-weight: 500;
		letter-spacing: -0.065em;
		line-height: 0.92;
	}

	.lede {
		max-width: 38rem;
		margin: 1.75rem 0 2.5rem;
		color: var(--color-muted-foreground);
		font-size: 1.05rem;
		line-height: 1.7;
	}

	.foundations {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
		margin-bottom: 2.5rem;
	}

	.foundation {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		padding: 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
	}

	.foundation span {
		color: var(--color-muted-foreground);
		font-size: 0.75rem;
		font-weight: 650;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	.foundation strong {
		font-size: 1rem;
		font-weight: 650;
	}

	.bridge {
		display: flex;
		flex-direction: column;
		gap: 0.8rem;
	}

	.section-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.section-label {
		margin: 0 0 0.3rem;
		color: var(--color-accent);
		font-size: 0.7rem;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	h2 {
		margin: 0;
		font-size: 1.1rem;
		font-weight: 600;
	}

	output {
		display: block;
		padding: 1rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		color: var(--color-foreground);
		font-family: var(--font-mono);
		font-size: 1rem;
	}

	.loading {
		color: var(--color-muted-foreground);
	}

	.loading,
	.error {
		margin: 0;
	}

	.error {
		color: var(--color-error);
	}

	@media (max-width: 600px) {
		main {
			padding: 1rem;
		}

		.hero {
			padding: 2rem 1.25rem;
			border-radius: var(--radius-xl);
		}

		.foundations {
			grid-template-columns: 1fr;
		}

	}
</style>
