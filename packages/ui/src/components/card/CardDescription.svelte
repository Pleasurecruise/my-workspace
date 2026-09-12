<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface CardDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
		ref?: HTMLParagraphElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: CardDescriptionProps = $props();
</script>

<p bind:this={ref} data-slot="card-description" class={cn("font-sans text-sm text-muted-foreground", className)} {...rest}>
	{@render children?.()}
</p>
