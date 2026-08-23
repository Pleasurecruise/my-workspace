<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface CardFooterProps extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: CardFooterProps = $props();
</script>

<div data-slot="footer" class={cn("flex items-center p-6 pt-0", className)} {...rest}>
	{@render children?.()}
</div>
