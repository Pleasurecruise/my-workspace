<script lang="ts" generics="T extends { id: string }">
	import { tick, type Snippet } from "svelte";
	import type { HTMLAttributes } from "svelte/elements";
	import { cn } from "../lib/classes";
	import Button from "./Button.svelte";

	let { items, label, itemLabel, children, onreorder, disabled = false, ref = $bindable(null), class: className = "", ...rest }: Omit<HTMLAttributes<HTMLUListElement>, "children"> & {
		items: T[];
		label: string;
		itemLabel: (item: T) => string;
		children: Snippet<[T]>;
		onreorder: (ids: string[]) => Promise<boolean>;
		disabled?: boolean;
		ref?: HTMLUListElement | null;
	} = $props();
	const id = $props.id();
	let dragging = $state<string | null>(null);
	let over = $state<string | null>(null);
	let pending = $state(false);
	let announcement = $state("");
	let error = $state<string | null>(null);
	let startY = 0;
	let pointerY = 0;
	let moved = false;
	let frame = 0;
	let originalIds: string[] = [];
	let handle: HTMLButtonElement | null = null;

	function cancel() {
		cancelAnimationFrame(frame);
		dragging = null;
		over = null;
	}

	$effect(() => {
		const ids = items.map((item) => item.id);
		if (disabled || ids.join("\0") !== originalIds.join("\0")) cancel();
		return cancel;
	});

	async function reorder(from: string, to: string, control: HTMLButtonElement) {
		if (disabled || pending || from === to) return;
		const ids = items.map((item) => item.id);
		const source = ids.indexOf(from);
		const destination = ids.indexOf(to);
		if (source < 0 || destination < 0) return;
		ids.splice(source, 1);
		ids.splice(destination, 0, from);
		pending = true;
		error = null;
		let saved = false;
		try {
			saved = await onreorder(ids);
		} catch {
			error = "Could not save the order. Try again.";
		} finally {
			pending = false;
		}
		await tick();
		if (control.isConnected) control.focus();
		const position = items.findIndex((item) => item.id === from);
		const item = items[position];
		if (item && saved) announcement = `${itemLabel(item)}, position ${position + 1} of ${items.length}.`;
	}

	function track() {
		if (dragging === null || ref === null) return;
		const bounds = ref.getBoundingClientRect();
		if (pointerY < bounds.top + 28) ref.scrollTop -= 6;
		else if (pointerY > bounds.bottom - 28) ref.scrollTop += 6;
		updateTarget();
		frame = requestAnimationFrame(track);
	}

	function updateTarget() {
		if (ref === null) return;
		for (const row of ref.children) {
			if (!(row instanceof HTMLElement)) continue;
			const rect = row.getBoundingClientRect();
			if (pointerY >= rect.top && pointerY <= rect.bottom) over = row.dataset.sortableId ?? null;
		}
	}
</script>

<p id={`${id}-instructions`} class="sr-only">Drag a handle to reorder. Use Up and Down arrow keys, Home or End to move a focused item. Escape cancels dragging.</p>
<ul bind:this={ref} data-slot="sortable-list" aria-label={label} aria-busy={pending} class={cn("sortable-list", className)} {...rest}>
	{#each items as item, index (item.id)}
		<li data-slot="sortable-item" data-sortable-id={item.id} data-dragging={dragging === item.id} data-over={over === item.id && dragging !== item.id}>
			<Button variant="ghost" size="icon-xs" class="sortable-handle" disabled={disabled || pending || items.length < 2} aria-label={`Reorder ${itemLabel(item)}`} aria-describedby={`${id}-instructions`} title={`Drag to reorder ${itemLabel(item)}`}
				onpointerdown={(event) => {
					if (event.button !== 0 || disabled || pending) return;
					event.stopPropagation();
					originalIds = items.map((entry) => entry.id);
					handle = event.currentTarget;
					handle.setPointerCapture(event.pointerId);
					startY = pointerY = event.clientY;
					moved = false;
					dragging = over = item.id;
				}}
				onpointermove={(event) => {
					if (dragging !== item.id) return;
					pointerY = event.clientY;
					if (!moved && Math.abs(pointerY - startY) > 4) { moved = true; frame = requestAnimationFrame(track); }
					if (moved) updateTarget();
				}}
				onpointerup={(event) => {
					pointerY = event.clientY;
					if (moved) updateTarget();
					const destination = over;
					const source = dragging;
					cancel();
					if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
					if (moved && source !== null && destination !== null && handle !== null) void reorder(source, destination, handle);
				}}
				onpointercancel={cancel} onlostpointercapture={cancel}
				onkeydown={(event) => {
					if (event.key === "Escape") { cancel(); return; }
					const destination = event.key === "ArrowUp" ? index - 1 : event.key === "ArrowDown" ? index + 1 : event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : -1;
					if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
					event.preventDefault(); event.stopPropagation();
					const target = items[destination];
					if (target) void reorder(item.id, target.id, event.currentTarget);
				}}>
				<svg width="12" height="16" viewBox="0 0 12 16" fill="currentColor" aria-hidden="true"><circle cx="4" cy="4" r="1"/><circle cx="8" cy="4" r="1"/><circle cx="4" cy="8" r="1"/><circle cx="8" cy="8" r="1"/><circle cx="4" cy="12" r="1"/><circle cx="8" cy="12" r="1"/></svg>
			</Button>
			{@render children(item)}
		</li>
	{/each}
</ul>
{#if error !== null}<p role="alert" class="text-sm text-error">{error}</p>{/if}
<p role="status" aria-live="polite" aria-atomic="true" class="sr-only">{announcement}</p>

<style>
	.sortable-list { display: grid; min-height: 0; align-content: start; gap: 0.15rem; margin: 0; padding: 0; overflow-y: auto; overscroll-behavior: contain; list-style: none; }
	li { display: flex; align-items: center; gap: 0.25rem; min-width: 0; border-radius: var(--radius-md); }
	li[data-dragging="true"] { background: var(--color-muted); opacity: 0.65; }
	li[data-over="true"] { box-shadow: inset 0 2px var(--color-accent); background: var(--color-muted); }
	li :global(.sortable-handle) { flex: 0 0 auto; touch-action: none; cursor: grab; color: var(--color-muted-foreground); }
	li[data-dragging="true"] :global(.sortable-handle) { cursor: grabbing; }
</style>
