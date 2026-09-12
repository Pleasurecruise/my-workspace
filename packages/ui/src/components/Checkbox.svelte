<script lang="ts">
	import type { HTMLInputAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";

	let { checked = $bindable(false), indeterminate = $bindable(false), ref = $bindable(null), size = "default", onCheckedChange, class: className = "", ...rest }: Omit<HTMLInputAttributes, "type" | "size" | "checked" | "indeterminate"> & {
		checked?: boolean;
		indeterminate?: boolean;
		onCheckedChange?: (checked: boolean) => void;
		ref?: HTMLInputElement | null;
		size?: "default" | "sm";
	} = $props();
</script>

<input bind:this={ref} data-slot="checkbox" type="checkbox" bind:checked={() => checked, (value) => { const next = value === true; if (onCheckedChange) { if (ref) ref.checked = checked; onCheckedChange(next); } else checked = next; }} bind:indeterminate class={cn("checkbox", size === "sm" && "small", className)} {...rest} />

<style>
	.checkbox { box-sizing: border-box; appearance: none; display: inline-grid; place-content: center; flex: 0 0 auto; width: 1rem; height: 1rem; margin: 0; border: 1px solid var(--color-border-strong); border-radius: var(--radius-sm); background: var(--color-background); color: var(--color-accent-foreground); cursor: pointer; }
	.checkbox.small { width: 0.75rem; height: 0.75rem; }
	.checkbox:checked, .checkbox:indeterminate { background: var(--color-accent); border-color: var(--color-accent); }
	.checkbox:checked::after { content: ""; width: 0.4rem; height: 0.2rem; border-left: 1.5px solid currentColor; border-bottom: 1.5px solid currentColor; transform: translateY(-1px) rotate(-45deg); }
	.checkbox:indeterminate::after { content: ""; width: 0.4rem; height: 0; border-left: 0; border-bottom: 1.5px solid currentColor; transform: none; }
	.checkbox:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.checkbox:disabled { cursor: not-allowed; opacity: 0.5; }
	.checkbox[aria-invalid="true"] { border-color: var(--color-error); outline-color: var(--color-error); }
</style>
