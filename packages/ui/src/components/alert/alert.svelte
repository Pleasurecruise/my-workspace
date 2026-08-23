<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export type AlertVariant = "default" | "success" | "warning" | "error";
	export interface AlertProps extends HTMLAttributes<HTMLDivElement> {
		variant?: AlertVariant;
		children?: Snippet;
	}

	const variants: Record<AlertVariant, string> = {
		default: "border-border bg-muted text-foreground",
		success: "border-success/30 bg-success/10 text-success",
		warning: "border-warning/30 bg-warning/10 text-warning",
		error: "border-error/30 bg-error/10 text-error",
	};

	let { variant = "default", class: className = "", children, ...rest }: AlertProps = $props();
</script>

<div role="alert" data-slot="root" class={cn("rounded-lg border px-4 py-3 font-sans text-sm", variants[variant], className)} {...rest}>
	{@render children?.()}
</div>
