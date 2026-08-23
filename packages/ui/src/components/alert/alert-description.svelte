<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../../lib/utils";

	export interface AlertDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
		children?: Snippet;
	}
	let { class: className = "", children, ...rest }: AlertDescriptionProps = $props();
</script>

<p data-slot="alert-description" class={cn("text-sm opacity-80", className)} {...rest}>
	{@render children?.()}
</p>
