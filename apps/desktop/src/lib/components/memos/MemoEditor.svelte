<script lang="ts">
	import { Hash } from "@lucide/svelte";
	import { Textarea } from "@my-workspace/ui";
	import { tick } from "svelte";

	let {
		value = $bindable(""),
		editorElement = $bindable(null),
		placeholder = "",
		tags = [],
		onsubmit,
	}: {
		value: string;
		editorElement?: HTMLTextAreaElement | null;
		placeholder: string;
		tags?: string[];
		onsubmit: () => void;
	} = $props();

	let query = $state("");
	let range = $state<{ start: number; end: number } | null>(null);
	let open = $state(false);
	let activeIndex = $state(0);
	let menuPosition = $state({ left: 0, top: 0 });
	let suggestions = $derived(
		tags.filter((tag) => tag.toLocaleLowerCase().includes(query.toLocaleLowerCase())).slice(0, 8),
	);

	$effect(() => {
		if (range !== null && value.slice(range.start, range.end) !== `#${query}`) {
			open = false;
			range = null;
		}
	});

	$effect(() => {
		const activeRange = range;
		if (!open || editorElement === null || activeRange === null) return;
		const editor = editorElement;
		const rangeEnd = activeRange.end;
		function positionMenu() {
			const mirror = document.createElement("div");
			const styles = getComputedStyle(editor);
			const rectangle = editor.getBoundingClientRect();
			mirror.style.cssText = `
				position: fixed; top: -9999px; left: 0; visibility: hidden;
				width: ${rectangle.width}px; min-height: ${rectangle.height}px;
				font: ${styles.font}; line-height: ${styles.lineHeight};
				padding: ${styles.padding}; border: ${styles.border};
				white-space: pre-wrap; overflow-wrap: break-word; box-sizing: ${styles.boxSizing};
				letter-spacing: ${styles.letterSpacing};
			`;
			mirror.textContent = editor.value.slice(0, rangeEnd);
			const marker = document.createElement("span");
			marker.textContent = "|";
			mirror.appendChild(marker);
			document.body.appendChild(mirror);
			menuPosition = {
				left: rectangle.left + marker.offsetLeft - editor.scrollLeft,
				top: rectangle.top + marker.offsetTop - editor.scrollTop + marker.offsetHeight,
			};
			mirror.remove();
		}
		positionMenu();
		window.addEventListener("scroll", positionMenu, { passive: true });
		window.addEventListener("resize", positionMenu);
		return () => {
			window.removeEventListener("scroll", positionMenu);
			window.removeEventListener("resize", positionMenu);
		};
	});

	function selectTag(tag: string) {
		if (range === null) return;
		const cursor = range.start + tag.length + 2;
		value = `${value.slice(0, range.start)}#${tag} ${value.slice(range.end)}`;
		open = false;
		range = null;
		void tick().then(() => {
			if (editorElement === null) return;
			editorElement.focus();
			editorElement.setSelectionRange(cursor, cursor);
		});
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.isComposing || event.keyCode === 229) return;
		if (event.currentTarget instanceof HTMLTextAreaElement) editorElement = event.currentTarget;
		if (open && event.key === "Escape") {
			open = false;
			return;
		}
		if (open && suggestions.length > 0 && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
			event.preventDefault();
			const offset = event.key === "ArrowDown" ? 1 : -1;
			activeIndex = (activeIndex + offset + suggestions.length) % suggestions.length;
			return;
		}
		if (open && suggestions.length > 0 && (event.key === "Enter" || event.key === "Tab")) {
			event.preventDefault();
			for (const [index, tag] of suggestions.entries()) {
				if (index === activeIndex) {
					selectTag(tag);
					break;
				}
			}
			return;
		}
		if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			onsubmit();
		}
	}

	function handleEditorEvent(event: Event) {
		if (!(event.currentTarget instanceof HTMLTextAreaElement)) return;
		const editor = event.currentTarget;
		editorElement = editor;
		value = editor.value;
		if (editor.selectionStart !== editor.selectionEnd) {
			open = false;
			range = null;
			return;
		}
		const position = editor.selectionEnd;
		const start = value.lastIndexOf("#", position - 1);
		if (start < 0) {
			open = false;
			range = null;
			return;
		}
		const fragment = value.slice(start + 1, position);
		if (!/^[\p{Letter}\p{Number}_\-/]*$/u.test(fragment)) {
			open = false;
			range = null;
			return;
		}
		if (range?.start === start && range.end === position && query === fragment) return;
		query = fragment;
		range = { start, end: position };
		activeIndex = 0;
		open = tags.length > 0;
	}
</script>

<Textarea
	class="min-h-20 resize-y border-0 bg-transparent px-3 py-2 text-sm leading-relaxed shadow-none [field-sizing:content] placeholder:opacity-85 focus-visible:ring-0 focus-visible:ring-offset-0"
	bind:value
	{placeholder}
	onfocus={(event: FocusEvent) => {
		if (event.currentTarget instanceof HTMLTextAreaElement) editorElement = event.currentTarget;
	}}
	oninput={handleEditorEvent}
	onclick={handleEditorEvent}
	onkeyup={handleEditorEvent}
	onselect={handleEditorEvent}
	onblur={() => window.setTimeout(() => (open = false), 150)}
	onkeydown={handleKeydown}
/>

{#if open && suggestions.length > 0}
	<div class="tag-suggestions" style:left={`${menuPosition.left}px`} style:top={`${menuPosition.top}px`} role="listbox" aria-label="Tag suggestions">
		{#each suggestions as tag, index (tag)}
			<button
				type="button"
				class:active={activeIndex === index}
				role="option"
				aria-selected={activeIndex === index}
				onmousedown={(event) => event.preventDefault()}
				onclick={() => selectTag(tag)}
			>
				<Hash size={13} /><span>{tag}</span>
			</button>
		{/each}
	</div>
{/if}

<style>
	.tag-suggestions {
		position: fixed;
		z-index: 90;
		display: grid;
		width: 10rem;
		max-height: 12rem;
		overflow-y: auto;
		padding: 0.25rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-background);
		box-shadow: var(--shadow-lg);
	}

	.tag-suggestions button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		min-width: 0;
		padding: 0.4rem 0.5rem;
		border: 0;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-muted-foreground);
		cursor: pointer;
		font-size: 0.75rem;
		text-align: left;
	}

	.tag-suggestions button.active,
	.tag-suggestions button:hover {
		background: var(--color-muted);
		color: var(--color-foreground);
	}

	.tag-suggestions span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
