<script lang="ts">
	import type { HTMLInputAttributes } from "svelte/elements";
	import { cn } from "../lib/utils";

	export interface InputProps extends HTMLInputAttributes {
		error?: boolean;
	}

	const base = cn(
		"flex h-9 w-full rounded-md border bg-background px-3 py-1",
		"font-sans text-sm text-foreground placeholder:text-muted-foreground",
		"outline-none",
		"focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
		"disabled:cursor-not-allowed disabled:opacity-50",
	);

	let { error, class: className = "", value = $bindable(), ...rest }: InputProps = $props();
</script>

<input
	class={cn(base, error ? "border-error focus-visible:ring-error" : "border-border focus-visible:ring-accent", className)}
	bind:value
	{...rest}
/>
