<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface CardHeaderProps extends HTMLAttributes<HTMLDivElement> {
		ref?: HTMLDivElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: CardHeaderProps = $props();
</script>

<div bind:this={ref} data-slot="card-header" class={cn("flex flex-col gap-1.5 p-6", className)} {...rest}>
	{@render children?.()}
</div>
