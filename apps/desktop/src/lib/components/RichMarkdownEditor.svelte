<script lang="ts">
	import type { Editor } from "@tiptap/core";
	import { Bold, Code, FileCode2, Heading2, Italic, Link, List, ListOrdered, Pilcrow, Quote, Redo2, Strikethrough, Undo2, Unlink } from "@lucide/svelte";
	import { onMount } from "svelte";

	let { value = $bindable(), maxLength = 500_000 }: { value: string; maxLength?: number } = $props();
	let element = $state<HTMLDivElement | null>(null);
	let editor = $state<Editor | null>(null);
	let length = $state(value.length);
	let mode = $state<"rich" | "source">("rich");
	let modeError = $state("");
	let toolbar = $state({
		heading: false,
		bold: false,
		italic: false,
		strike: false,
		bulletList: false,
		orderedList: false,
		blockquote: false,
		codeBlock: false,
		link: false,
	});

	function representsSource(markdown: string) {
		return markdown.replaceAll("\r\n", "\n").trimEnd() === value.replaceAll("\r\n", "\n").trimEnd();
	}

	function syncToolbar(current: Editor) {
		toolbar = {
			heading: current.isActive("heading", { level: 2 }),
			bold: current.isActive("bold"),
			italic: current.isActive("italic"),
			strike: current.isActive("strike"),
			bulletList: current.isActive("bulletList"),
			orderedList: current.isActive("orderedList"),
			blockquote: current.isActive("blockquote"),
			codeBlock: current.isActive("codeBlock"),
			link: current.isActive("link"),
		};
	}

	onMount(() => {
		if (element === null) return;
		const editorElement = element;
		let disposed = false;
		let instance: Editor | null = null;
		void Promise.all([import("@tiptap/core"), import("@tiptap/markdown"), import("@tiptap/starter-kit")]).then(
			([core, markdown, starterKit]) => {
				if (disposed) return;
				instance = new core.Editor({
					element: editorElement,
					extensions: [starterKit.default, markdown.Markdown],
					content: value,
					contentType: "markdown",
					onUpdate: ({ editor: current }) => {
						const source = current.getMarkdown();
						value = source;
						length = source.length;
						syncToolbar(current);
					},
					onSelectionUpdate: ({ editor: current }) => syncToolbar(current),
				});
				editor = instance;
				if (!representsSource(instance.getMarkdown())) {
					mode = "source";
					modeError = "Rich text mode cannot represent every construct in this Markdown. Source mode is preserving it exactly.";
				}
				syncToolbar(instance);
			},
			() => {
				if (disposed) return;
				mode = "source";
				modeError = "The rich text editor could not be loaded. Markdown source remains available.";
			},
		);
		return () => {
			disposed = true;
			if (instance !== null) instance.destroy();
		};
	});

	function enterRichMode() {
		if (editor === null) return;
		editor.commands.setContent(value, { contentType: "markdown", emitUpdate: false });
		if (!representsSource(editor.getMarkdown())) {
			modeError = "Rich text mode would rewrite or remove part of this Markdown. Keep editing it in source mode.";
			return;
		}
		modeError = "";
		mode = "rich";
		syncToolbar(editor);
	}

	function toggleLink() {
		if (editor === null) return;
		if (editor.isActive("link")) {
			editor.chain().focus().unsetLink().run();
			return;
		}
		const href = window.prompt("Link URL", "https://");
		if (href === null || href.trim() === "") return;
		let url: URL;
		try {
			url = new URL(href.trim());
		} catch {
			window.alert("Enter a valid HTTP or HTTPS URL.");
			return;
		}
		if (url.protocol !== "http:" && url.protocol !== "https:") {
			window.alert("Only HTTP and HTTPS links are supported.");
			return;
		}
		editor.chain().focus().extendMarkRange("link").setLink({ href: url.href }).run();
	}
</script>

<div class="rich-editor">
	<div class="mode-switch" role="group" aria-label="Editor mode">
		<button type="button" class:active={mode === "rich"} onclick={enterRichMode}><Pilcrow size={14} /> Rich text</button>
		<button type="button" class:active={mode === "source"} onclick={() => { mode = "source"; modeError = ""; }}><FileCode2 size={14} /> Markdown</button>
	</div>
	{#if modeError}<p class="mode-error" role="status">{modeError}</p>{/if}
	<div class:hidden={mode !== "rich"} class="toolbar" role="toolbar" aria-label="Article formatting">
		<button type="button" class:active={toolbar.heading} onclick={() => editor?.chain().focus().toggleHeading({ level: 2 }).run()} aria-label="Heading" title="Heading"><Heading2 size={15} /></button>
		<button type="button" class:active={toolbar.bold} onclick={() => editor?.chain().focus().toggleBold().run()} aria-label="Bold" title="Bold"><Bold size={15} /></button>
		<button type="button" class:active={toolbar.italic} onclick={() => editor?.chain().focus().toggleItalic().run()} aria-label="Italic" title="Italic"><Italic size={15} /></button>
		<button type="button" class:active={toolbar.strike} onclick={() => editor?.chain().focus().toggleStrike().run()} aria-label="Strikethrough" title="Strikethrough"><Strikethrough size={15} /></button>
		<span></span>
		<button type="button" class:active={toolbar.bulletList} onclick={() => editor?.chain().focus().toggleBulletList().run()} aria-label="Bullet list" title="Bullet list"><List size={15} /></button>
		<button type="button" class:active={toolbar.orderedList} onclick={() => editor?.chain().focus().toggleOrderedList().run()} aria-label="Numbered list" title="Numbered list"><ListOrdered size={15} /></button>
		<button type="button" class:active={toolbar.blockquote} onclick={() => editor?.chain().focus().toggleBlockquote().run()} aria-label="Quote" title="Quote"><Quote size={15} /></button>
		<button type="button" class:active={toolbar.codeBlock} onclick={() => editor?.chain().focus().toggleCodeBlock().run()} aria-label="Code block" title="Code block"><Code size={15} /></button>
		<span></span>
		<button type="button" class:active={toolbar.link} onclick={toggleLink} aria-label={toolbar.link ? "Remove link" : "Add link"} title={toolbar.link ? "Remove link" : "Add link"}>{#if toolbar.link}<Unlink size={15} />{:else}<Link size={15} />{/if}</button>
		<button type="button" onclick={() => editor?.chain().focus().undo().run()} disabled={!editor?.can().undo()} aria-label="Undo" title="Undo"><Undo2 size={15} /></button>
		<button type="button" onclick={() => editor?.chain().focus().redo().run()} disabled={!editor?.can().redo()} aria-label="Redo" title="Redo"><Redo2 size={15} /></button>
		<small class:limit={length >= maxLength}>{length.toLocaleString()} / {maxLength.toLocaleString()}</small>
	</div>
	<div class:hidden={mode !== "rich"} class="surface" bind:this={element}></div>
	{#if mode === "source"}
		<textarea bind:value maxlength={maxLength} oninput={() => (length = value.length)} aria-label="Article Markdown source" spellcheck="true"></textarea>
		<small class:limit={length >= maxLength} class="source-length">{length.toLocaleString()} / {maxLength.toLocaleString()}</small>
	{/if}
</div>

<style>
	.rich-editor { overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); }
	.rich-editor:focus-within { border-color: var(--color-border-strong); box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 16%, transparent); }
	.mode-switch { display: flex; gap: 0.2rem; padding: 0.4rem; border-bottom: 1px solid var(--color-border); background: var(--color-muted); }
	.mode-switch button { display: inline-flex; height: 1.8rem; align-items: center; gap: 0.35rem; padding: 0 0.55rem; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; font-size: 0.68rem; }
	.mode-switch button.active { background: var(--color-background); color: var(--color-foreground); box-shadow: var(--shadow-xs); }
	.mode-error { margin: 0; padding: 0.6rem 0.75rem; border-bottom: 1px solid var(--color-border); color: var(--color-error); font-family: var(--font-sans); font-size: 0.7rem; letter-spacing: normal; line-height: 1.45; text-transform: none; }
	.toolbar { position: sticky; top: 0; z-index: 2; display: flex; flex-wrap: wrap; align-items: center; gap: 0.2rem; padding: 0.45rem; border-bottom: 1px solid var(--color-border); background: color-mix(in srgb, var(--color-background) 94%, transparent); backdrop-filter: blur(10px); }
	.hidden { display: none; }
	.toolbar button { display: grid; width: 1.9rem; height: 1.9rem; place-items: center; padding: 0; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted-foreground); cursor: pointer; }
	.toolbar button:hover:not(:disabled) { background: var(--color-muted); color: var(--color-foreground); }
	.toolbar button.active { background: color-mix(in srgb, var(--color-accent) 12%, transparent); color: var(--color-accent); }
	.toolbar button:disabled { cursor: default; opacity: 0.35; }
	.toolbar > span { width: 1px; height: 1.25rem; margin: 0 0.2rem; background: var(--color-border); }
	.toolbar small { margin-left: auto; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.6rem; }
	.toolbar small.limit { color: var(--color-error); }
	.rich-editor > textarea { display: block; width: 100%; min-height: calc(100vh - 21rem); box-sizing: border-box; padding: 1rem 1.15rem 3rem; resize: vertical; border: 0; outline: none; background: var(--color-background); color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.85rem; line-height: 1.7; }
	.source-length { display: block; padding: 0 0.75rem 0.6rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.6rem; text-align: right; }
	.source-length.limit { color: var(--color-error); }
	.surface { min-height: calc(100vh - 21rem); max-height: calc(100vh - 14rem); overflow-y: auto; }
	.surface :global(.tiptap) { min-height: calc(100vh - 21rem); padding: 1rem 1.15rem 5rem; outline: none; color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.9rem; line-height: 1.7; }
	.surface :global(.tiptap > *:first-child) { margin-top: 0; }
	.surface :global(.tiptap h1), .surface :global(.tiptap h2), .surface :global(.tiptap h3) { margin: 1.5em 0 0.6em; font-weight: 600; line-height: 1.35; }
	.surface :global(.tiptap h1) { font-size: 1.6em; }
	.surface :global(.tiptap h2) { padding-bottom: 0.25em; border-bottom: 1px solid var(--color-border); font-size: 1.35em; }
	.surface :global(.tiptap h3) { font-size: 1.15em; }
	.surface :global(.tiptap p), .surface :global(.tiptap ul), .surface :global(.tiptap ol), .surface :global(.tiptap blockquote), .surface :global(.tiptap pre) { margin: 0.8em 0; }
	.surface :global(.tiptap ul), .surface :global(.tiptap ol) { padding-left: 1.5rem; }
	.surface :global(.tiptap blockquote) { padding-left: 1rem; border-left: 3px solid var(--color-border-strong); color: var(--color-muted-foreground); }
	.surface :global(.tiptap pre) { overflow-x: auto; padding: 0.875rem 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-muted); font-family: var(--font-mono); }
	.surface :global(.tiptap code) { font-family: var(--font-mono); }
	.surface :global(.tiptap a) { color: var(--color-accent); text-decoration: underline; text-underline-offset: 0.2em; }
</style>
