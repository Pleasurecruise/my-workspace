<script lang="ts">
	import type { HTMLTextareaAttributes } from "svelte/elements";
	import { cn } from "../lib/utils";

	export interface TextareaProps extends HTMLTextareaAttributes {
		error?: boolean;
	}

	const base = cn(
		"flex min-h-20 w-full resize-y rounded-md border bg-background px-3 py-2",
		"font-sans text-sm text-foreground placeholder:text-muted-foreground",
		"outline-none transition-colors duration-100",
		"focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
		"disabled:cursor-not-allowed disabled:opacity-50",
	);

	let { error, class: className = "", value = $bindable(), ...rest }: TextareaProps = $props();
</script>

<textarea
	class={cn(base, error ? "border-error focus-visible:ring-error" : "border-border focus-visible:ring-accent", className)}
	bind:value
	{...rest}
></textarea>
