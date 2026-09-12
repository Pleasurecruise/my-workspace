<script lang="ts">
	import type { HTMLTextareaAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";

	export interface TextareaProps extends HTMLTextareaAttributes {
		ref?: HTMLTextAreaElement | null;
		error?: boolean;
	}

	const base = cn(
		"flex min-h-20 w-full resize-y rounded-md border bg-background px-3 py-2",
		"font-sans text-sm text-foreground placeholder:text-muted-foreground",
		"outline-none aria-invalid:border-error aria-invalid:ring-2 aria-invalid:ring-error/20",
		"focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
		"disabled:cursor-not-allowed disabled:opacity-50",
	);

	let { ref = $bindable(null), error, "aria-invalid": invalid = error, class: className = "", value = $bindable(), ...rest }: TextareaProps = $props();
</script>

<textarea bind:this={ref} data-slot="textarea"
	class={cn(base, error ? "border-error focus-visible:ring-error" : "border-border focus-visible:ring-accent", className)}
	aria-invalid={invalid}
	bind:value
	{...rest}
></textarea>
