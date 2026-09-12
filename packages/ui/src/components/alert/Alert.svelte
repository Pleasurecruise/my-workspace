<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export type AlertVariant = "default" | "success" | "warning" | "error" | "destructive";
	export interface AlertProps extends HTMLAttributes<HTMLDivElement> {
		ref?: HTMLDivElement | null;
		variant?: AlertVariant;
		children?: Snippet;
	}

	const variants: Record<AlertVariant, string> = {
		destructive: "border-error/30 bg-error/10 text-error",
		default: "border-border bg-muted text-foreground",
		success: "border-success/30 bg-success/10 text-success",
		warning: "border-warning/30 bg-warning/10 text-warning",
		error: "border-error/30 bg-error/10 text-error",
	};

	let { ref = $bindable(null), variant = "default", class: className = "", children, ...rest }: AlertProps = $props();
</script>

<div bind:this={ref} data-slot="alert" role="alert" class={cn("rounded-lg border px-4 py-3 font-sans text-sm", variants[variant], className)} {...rest}>
	{@render children?.()}
</div>
