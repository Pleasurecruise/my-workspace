<script lang="ts">
	let { configuration, error }: { configuration: string; error: string } = $props();
</script>

<section aria-label="Invalid widget">
	<h2>Invalid widget</h2>
	<p>This card could not be loaded. Remove it in Edit dashboard and add it again.</p>
	<pre aria-label="Widget configuration">{configuration}</pre>
	<details><summary>Error details</summary><p>{error}</p></details>
</section>

<style>
	section { padding: 0.85rem; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-background); }
	h2 { margin: 0; font-size: 0.75rem; color: var(--color-error); }
	p, summary { font-size: 0.7rem; color: var(--color-muted-foreground); overflow-wrap: anywhere; }
	pre { max-height: 10rem; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; font: 0.7rem var(--font-mono); }
	summary { cursor: pointer; }
</style>
