<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface CardTitleProps extends HTMLAttributes<HTMLHeadingElement> {
		ref?: HTMLHeadingElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: CardTitleProps = $props();
</script>

<h3 bind:this={ref} data-slot="card-title" class={cn("font-sans text-base font-semibold leading-none text-foreground", className)} {...rest}>
	{@render children?.()}
</h3>
