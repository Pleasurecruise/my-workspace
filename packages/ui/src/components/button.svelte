<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLButtonAttributes } from "svelte/elements";
	import { cva, type VariantProps } from "class-variance-authority";
	import { cn } from "../lib/utils";

	const buttonVariants = cva(
		"inline-flex h-10 items-center justify-center rounded-md px-5 text-sm font-medium transition disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
		{
			variants: {
				variant: {
					default: "bg-accent text-accent-foreground hover:opacity-90",
					outline: "border border-border bg-background hover:bg-muted",
					ghost: "hover:bg-muted",
				},
			},
			defaultVariants: { variant: "default" },
		},
	);

	type Props = HTMLButtonAttributes &
		VariantProps<typeof buttonVariants> & {
			children?: Snippet;
		};

	let {
		class: className,
		variant = "default",
		children,
		...rest
	}: Props = $props();
</script>

<button class={cn(buttonVariants({ variant }), className)} {...rest}>
	{@render children?.()}
</button>
