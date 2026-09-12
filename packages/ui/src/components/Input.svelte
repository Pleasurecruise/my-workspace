<script lang="ts">
	import type { HTMLInputAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";

	export interface InputProps extends HTMLInputAttributes {
		ref?: HTMLInputElement | null;
		error?: boolean;
	}

	const base = cn(
		"flex h-9 w-full rounded-md border bg-background px-3 py-1",
		"font-sans text-sm text-foreground placeholder:text-muted-foreground",
		"outline-none aria-invalid:border-error aria-invalid:ring-2 aria-invalid:ring-error/20",
		"focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
		"disabled:cursor-not-allowed disabled:opacity-50",
	);

	let { ref = $bindable(null), error, "aria-invalid": invalid = error, class: className = "", value = $bindable(), ...rest }: InputProps = $props();
</script>

<input bind:this={ref} data-slot="input"
	class={cn(base, error ? "border-error focus-visible:ring-error" : "border-border focus-visible:ring-accent", className)}
	aria-invalid={invalid}
	bind:value
	{...rest}
/>
