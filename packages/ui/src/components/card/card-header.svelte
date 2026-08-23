<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface CardHeaderProps extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: CardHeaderProps = $props();
</script>

<div data-slot="header" class={cn("flex flex-col gap-1.5 p-6", className)} {...rest}>
	{@render children?.()}
</div>
