<script lang="ts">
	import type { HTMLInputAttributes } from "svelte/elements";
	import { cn } from "../lib/utils";

	let { class: className, value = $bindable(), ...rest }: HTMLInputAttributes = $props();
</script>

<input
	class={cn(
		"h-11 w-full rounded-md border border-border bg-background px-4 text-sm text-foreground shadow-xs outline-none transition placeholder:text-muted-foreground focus:border-border-strong focus:ring-2 focus:ring-accent/20",
		className,
	)}
	bind:value
	{...rest}
/>
