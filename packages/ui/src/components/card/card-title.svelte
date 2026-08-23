<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface CardTitleProps extends HTMLAttributes<HTMLHeadingElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: CardTitleProps = $props();
</script>

<h3 data-slot="title" class={cn("font-sans text-base font-semibold leading-none text-foreground", className)} {...rest}>
	{@render children?.()}
</h3>
