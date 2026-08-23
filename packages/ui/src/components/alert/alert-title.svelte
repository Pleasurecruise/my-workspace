<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface AlertTitleProps extends HTMLAttributes<HTMLParagraphElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: AlertTitleProps = $props();
</script>

<p data-slot="alert-title" class={cn("mb-1 font-medium leading-none", className)} {...rest}>
	{@render children?.()}
</p>
