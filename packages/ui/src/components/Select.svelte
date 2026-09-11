<script lang="ts">
	let { value, options, label, disabled = false, size = "default", onchange }: {
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
	let trigger = $state<HTMLButtonElement | null>(null);
	let list = $state<HTMLDivElement | null>(null);
	$effect(() => {
		if (disabled || options.length === 0) open = false;
		focused = Math.max(0, Math.min(focused, options.length - 1));
	});
	$effect(() => {
		const option = list?.children.item(focused);
		if (open && option instanceof HTMLElement) option.scrollIntoView({ block: "nearest" });
	});
	let selected = $derived(options.find((option) => option.value === value));
	function toggle() {
		if (disabled || options.length === 0) return;
		focused = Math.max(0, options.findIndex((option) => option.value === value));
		open = !open;
	}
	function choose(index: number) {
		const option = options[index];
		if (disabled) return;
		if (option) onchange(option.value);
		open = false;
		trigger?.focus();
	}
</script>

<svelte:window onpointerdown={(event) => { if (event.target instanceof Node && !trigger?.parentElement?.contains(event.target)) open = false; }} />
<div class="select" class:compact={size === "compact"}>
	<button bind:this={trigger} type="button" role="combobox" aria-label={label} aria-haspopup="listbox" aria-expanded={open} aria-controls={`${id}-list`} aria-activedescendant={open && options.length > 0 ? `${id}-${focused}` : ""} {disabled} onclick={toggle}
		onkeydown={(event) => {
			if (event.key === "Escape" || event.key === "Tab") { open = false; return; }
			if (disabled || options.length === 0) return;
			if (event.key === "ArrowDown" || event.key === "ArrowUp") {
				event.preventDefault();
				if (!open) { toggle(); return; }
				focused = (focused + (event.key === "ArrowDown" ? 1 : -1) + options.length) % options.length;
			} else if (event.key === "Home" || event.key === "End") {
				event.preventDefault(); open = true; focused = event.key === "Home" ? 0 : options.length - 1;
			} else if (open && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); choose(focused); }
		}}>
		<span>{selected ? selected.label : label}</span><svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="m3 4.5 3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
	</button>
	{#if open && !disabled}
		<div bind:this={list} class="options" id={`${id}-list`} role="listbox" aria-label={label}>
			{#each options as option, index (option.value)}
				<button type="button" id={`${id}-${index}`} role="option" aria-selected={option.value === value} tabindex="-1" class:focused={focused === index} onpointerdown={(event) => event.preventDefault()} onclick={() => choose(index)}>{option.label}<span aria-hidden="true">{option.value === value ? "✓" : ""}</span></button>
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
