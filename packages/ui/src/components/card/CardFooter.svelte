<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface CardFooterProps extends HTMLAttributes<HTMLDivElement> {
		ref?: HTMLDivElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: CardFooterProps = $props();
</script>

<div bind:this={ref} data-slot="card-footer" class={cn("flex items-center p-6 pt-0", className)} {...rest}>
	{@render children?.()}
</div>
