<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLLabelAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";

	export interface LabelProps extends HTMLLabelAttributes {
		ref?: HTMLLabelElement | null;
		required?: boolean;
		children?: Snippet;
	}

	let { ref = $bindable(null), required, class: className = "", children, ...rest }: LabelProps = $props();
</script>

<label bind:this={ref} data-slot="label"
	class={cn(
		"font-sans text-sm font-medium leading-none text-foreground",
		"peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
		required && "after:ml-1 after:text-error after:content-['*']",
		className,
	)}
	{...rest}
>
	{@render children?.()}
</label>
