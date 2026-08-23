<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface CardDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: CardDescriptionProps = $props();
</script>

<p data-slot="description" class={cn("font-sans text-sm text-muted-foreground", className)} {...rest}>
	{@render children?.()}
</p>
