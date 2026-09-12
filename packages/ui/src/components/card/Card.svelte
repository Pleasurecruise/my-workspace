<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface CardProps extends HTMLAttributes<HTMLDivElement> {
		ref?: HTMLDivElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: CardProps = $props();
</script>

<div bind:this={ref} data-slot="card" class={cn("rounded-lg border border-border bg-background shadow-xs", className)} {...rest}>
	{@render children?.()}
</div>
