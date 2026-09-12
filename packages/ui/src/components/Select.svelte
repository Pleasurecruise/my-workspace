<script lang="ts">
	import { cn } from "../lib/classes";
	let { value, options, label, disabled = false, size = "default", onchange, ref = $bindable(null), class: className = "" }: {
		ref?: HTMLButtonElement | null;
		class?: string;
		value: string;
		options: Array<{ value: string; label: string }>;
		label: string;
		disabled?: boolean;
		size?: "default" | "compact";
		onchange: (value: string) => void;
	} = $props();
	const id = $props.id();
	let open = $state(false);
	let focused = $state(0);
	let list = $state<HTMLDivElement | null>(null);
	$effect(() => {
		if (disabled || options.length === 0) open = false;
		focused = Math.max(0, Math.min(focused, options.length - 1));
	});
	$effect(() => {
		const option = list?.children.item(focused);
		if (open && option instanceof HTMLElement) option.scrollIntoView({ block: "nearest" });
	});
	let search = "";
	let searchedAt = 0;
	let selected = $derived(options.find((option) => option.value === value));
	function toggle() {
		if (disabled || options.length === 0) return;
		search = "";
		focused = Math.max(0, options.findIndex((option) => option.value === value));
		open = !open;
	}
	function choose(index: number) {
		const option = options[index];
		if (disabled) return;
		if (option) onchange(option.value);
		open = false;
		search = "";
		ref?.focus();
	}
</script>

<svelte:window onpointerdown={(event) => { if (event.target instanceof Node && !ref?.parentElement?.contains(event.target)) open = false; }} />
<div data-slot="select" class={cn("select", className)} class:compact={size === "compact"}>
	<button data-slot="select-trigger" bind:this={ref} type="button" role="combobox" aria-label={label} aria-haspopup="listbox" aria-expanded={open} aria-controls={open ? `${id}-list` : undefined} aria-activedescendant={open && options.length > 0 ? `${id}-${focused}` : ""} disabled={disabled || options.length === 0} onclick={toggle}
		onkeydown={(event) => {
			if (event.key === "Escape" || event.key === "Tab") { open = false; search = ""; return; }
			if (disabled || options.length === 0) return;
			if (event.key === "ArrowDown" || event.key === "ArrowUp") {
				event.preventDefault();
				if (!open) { toggle(); return; }
				focused = (focused + (event.key === "ArrowDown" ? 1 : -1) + options.length) % options.length;
			} else if (event.key === "Home" || event.key === "End") {
				event.preventDefault(); open = true; focused = event.key === "Home" ? 0 : options.length - 1;
			} else if (open && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); choose(focused); }
			else if (event.key.length === 1 && event.key !== " " && !event.ctrlKey && !event.metaKey && !event.altKey && !event.isComposing) {
				const now = Date.now();
				const key = event.key.toLocaleLowerCase();
				search = now - searchedAt > 700 ? key : search + key;
				searchedAt = now;
				const query = [...search].every((character) => character === search[0]) ? key : search;
				const start = open ? focused : options.findIndex((option) => option.value === value);
				for (let offset = query.length === 1 ? 1 : 0; offset <= options.length; offset += 1) {
					const index = (start + offset + options.length) % options.length;
					const option = options[index];
					if (!option || !option.label.toLocaleLowerCase().startsWith(query)) continue;
					event.preventDefault();
					if (open) focused = index; else onchange(option.value);
					break;
				}
			}
		}}>
		<span>{selected ? selected.label : label}</span><svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="m3 4.5 3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
	</button>
	{#if open && !disabled}
		<div bind:this={list} class="options" data-slot="select-content" id={`${id}-list`} role="listbox" aria-label={label}>
			{#each options as option, index (option.value)}
				<button data-slot="select-item" type="button" id={`${id}-${index}`} role="option" aria-selected={option.value === value} tabindex="-1" class:focused={focused === index} onpointerdown={(event) => event.preventDefault()} onclick={() => choose(index)}>{option.label}<span aria-hidden="true">{option.value === value ? "✓" : ""}</span></button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.select { position: relative; min-width: 0; }
	button { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; width: 100%; padding: 0.55rem 0.7rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); color: var(--color-foreground); font: inherit; font-size: 0.8rem; box-sizing: border-box; min-height: 2.25rem; cursor: pointer; text-align: left; }
	button > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	button:disabled { opacity: 0.5; cursor: default; }
	button:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.compact > button { box-sizing: border-box; height: 2rem; min-height: 2rem; padding: 0.45rem 0.55rem; font: 0.72rem var(--font-sans); }
	.options { box-sizing: border-box; position: absolute; z-index: 50; top: calc(100% + 0.35rem); left: 0; right: 0; max-height: 16rem; overflow-y: auto; overscroll-behavior: contain; padding: 0.3rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); box-shadow: var(--shadow-lg); }
	.compact .options { max-height: 10rem; }
	.compact .options button { min-height: 2rem; padding: 0.4rem 0.55rem; font: 0.72rem var(--font-sans); }
	.options button { border: 0; }
	.options button:hover, .options button.focused { background: var(--color-muted); }
	svg { flex-shrink: 0; }
</style>
