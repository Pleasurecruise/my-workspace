<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLButtonAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";

	export type ButtonVariant = "default" | "secondary" | "outline" | "ghost" | "destructive" | "link";
	export type ButtonSize = "default" | "xs" | "sm" | "md" | "lg" | "icon" | "icon-xs" | "icon-sm" | "icon-lg";

	export interface ButtonProps extends HTMLButtonAttributes {
		ref?: HTMLButtonElement | null;
		variant?: ButtonVariant;
		size?: ButtonSize;
		children?: Snippet;
	}

	const base = cn(
		"inline-flex items-center justify-center gap-2",
		"rounded-md font-sans text-sm font-medium",
		"cursor-pointer select-none whitespace-nowrap",
		"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background",
		"disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-error aria-invalid:ring-error/20 [&_svg]:pointer-events-none [&_svg]:shrink-0",
	);

	const variants: Record<ButtonVariant, string> = {
		default: "bg-accent text-accent-foreground hover:bg-accent/85",
		secondary: "border border-border bg-muted text-foreground hover:bg-border",
		outline: "border border-border bg-transparent text-foreground hover:bg-muted",
		ghost: "bg-transparent text-foreground hover:bg-muted",
		destructive: "bg-error text-error-foreground hover:enabled:bg-error-hover",
		link: "text-accent underline-offset-4 hover:underline",
	};

	const sizes: Record<ButtonSize, string> = {
		default: "h-9 px-4 text-sm",
		xs: "h-6 px-2 text-xs",
		"icon-xs": "size-6 p-0",
		"icon-sm": "size-8 p-0",
		"icon-lg": "size-10 p-0",
		sm: "h-8 px-3 text-xs",
		md: "h-9 px-4 text-sm",
		lg: "h-10 px-6 text-base",
		icon: "h-9 w-9 p-0",
	};

	let { ref = $bindable(null), variant = "default", size = "default", type = "button", class: className = "", children, ...rest }: ButtonProps = $props();
</script>

<button bind:this={ref} data-slot="button" {type} class={cn(base, variants[variant], sizes[size], className)} {...rest}>
	{@render children?.()}
</button>
