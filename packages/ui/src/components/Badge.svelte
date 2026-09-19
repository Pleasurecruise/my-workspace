<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";

	export type BadgeVariant = "default" | "secondary" | "success" | "warning" | "destructive" | "outline" | "accent-outline";

	export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
		ref?: HTMLSpanElement | null;
		variant?: BadgeVariant;
		size?: "default" | "sm";
		children?: Snippet;
	}

	const base = "inline-flex items-center rounded-full py-0.5 font-medium font-sans";
	const sizes = { default: "px-2 text-xs", sm: "px-2.5 text-[0.68rem]" };

	const variants: Record<BadgeVariant, string> = {
		default: "bg-accent text-accent-foreground",
		secondary: "bg-border text-foreground",
		success: "bg-success/15 text-success",
		warning: "bg-warning/15 text-warning",
		destructive: "bg-error/15 text-error",
		outline: "border border-border text-foreground",
		"accent-outline": "border border-accent/25 bg-transparent text-accent font-normal",
	};

	let { ref = $bindable(null), variant = "default", size = "default", class: className = "", children, ...rest }: BadgeProps = $props();
</script>

<span bind:this={ref} data-slot="badge" class={cn(base, variants[variant], sizes[size], className)} {...rest}>
	{@render children?.()}
</span>
