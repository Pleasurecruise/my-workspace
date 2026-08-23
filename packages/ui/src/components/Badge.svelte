<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../lib/utils";

	export type BadgeVariant = "default" | "secondary" | "success" | "warning" | "destructive" | "outline";

	export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
		variant?: BadgeVariant;
		children?: Snippet;
	}

	const base = "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium font-sans";

	const variants: Record<BadgeVariant, string> = {
		default: "bg-accent text-accent-foreground",
		secondary: "bg-border text-foreground",
		success: "bg-success/15 text-success",
		warning: "bg-warning/15 text-warning",
		destructive: "bg-error/15 text-error",
		outline: "border border-border text-foreground",
	};

	let { variant = "default", class: className = "", children, ...rest }: BadgeProps = $props();
</script>

<span class={cn(base, variants[variant], className)} {...rest}>
	{@render children?.()}
</span>
