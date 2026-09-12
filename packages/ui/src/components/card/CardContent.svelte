<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface CardContentProps extends HTMLAttributes<HTMLDivElement> {
		ref?: HTMLDivElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: CardContentProps = $props();
</script>

<div bind:this={ref} data-slot="card-content" class={cn("p-6 pt-0", className)} {...rest}>
	{@render children?.()}
</div>
