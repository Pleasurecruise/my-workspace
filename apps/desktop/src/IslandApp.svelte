<script lang="ts">
	import DynamicIsland from "./lib/components/dashboard/DynamicIsland.svelte";
	import { createDashboardSession } from "./lib/components/dashboard/session.svelte";
	import { createLayoutSession } from "./lib/components/dashboard/layout.svelte";
	const layoutSession = createLayoutSession();
	const session = createDashboardSession(() => false, "island");
</script>

{#key layoutSession.layout?.islandWidgetId}
	<DynamicIsland {session} {layoutSession} />
{/key}

<style>
    :global(html), :global(body), :global(#app) {
        width: 100%; height: 100%; min-width: 0; min-height: 0;
        margin: 0; padding: 0; overflow: hidden; background: transparent;
        font-family: var(--font-sans);
    }
</style>
