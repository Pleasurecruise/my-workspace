<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface CardContentProps extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: CardContentProps = $props();
</script>

<div data-slot="content" class={cn("p-6 pt-0", className)} {...rest}>
	{@render children?.()}
</div>
