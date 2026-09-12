<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/classes";

	export interface AlertDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
		ref?: HTMLParagraphElement | null;
		children?: Snippet;
	}
	let { ref = $bindable(null), class: className = "", children, ...rest }: AlertDescriptionProps = $props();
</script>

<p bind:this={ref} data-slot="alert-description" class={cn("text-sm opacity-80", className)} {...rest}>
	{@render children?.()}
</p>
