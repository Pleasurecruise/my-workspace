<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface CardProps extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: CardProps = $props();
</script>

<div data-slot="root" class={cn("rounded-lg border border-border bg-background shadow-xs", className)} {...rest}>
	{@render children?.()}
</div>
