<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { Bold, Code, FileCode2, Heading2, Italic, Link, List, ListOrdered, Pilcrow, Quote, Redo2, Strikethrough, Undo2, Unlink, Table, ListChecks, ImagePlus } from "@lucide/svelte";
	import { onMount } from "svelte";
	import type { MarkdownSpan } from "../../consumer";
	import type { createEditor, readSelection } from "./milkdown";

	let { value = $bindable(), maxLength = 500_000 }: { value: string; maxLength?: number } = $props();
	let element = $state<HTMLDivElement | null>(null);
	let editor = $state<Awaited<ReturnType<typeof createEditor>> | null>(null);
	let length = $derived(value.length);
	let mode = $state<"rich" | "source">("source");
	let modeRequest = 0;
	let disposed = false;
	let checking = $state(false);
	let modeError = $state("");
	let toolbar = $state<ReturnType<typeof readSelection>>({
		heading: false, bold: false, italic: false, strike: false,
		bulletList: false, orderedList: false, blockquote: false,
		codeBlock: false, link: false, sourceBlock: false, canUndo: false, canRedo: false,
	});

	onMount(() => {
		if (element === null) return;
		const editorElement = element;
		let mountedEditor: Awaited<ReturnType<typeof createEditor>> | null = null;
		void import("./milkdown").then(async (milkdown) => {
			if (disposed) return;
			mountedEditor = await milkdown.createEditor(editorElement, {
				maxLength,
				onChange: (source) => { if (mode === "rich") { value = source; modeError = ""; } },
				onSelection: (view) => { if (!disposed) toolbar = milkdown.readSelection(view); },
				onLimit: () => { modeError = "The article has reached its character limit."; },
			});
			if (disposed) { await mountedEditor.milkdown.destroy(); return; }
			editor = mountedEditor;
			if (modeRequest === 0) await enterRichMode();
		}).catch((error) => {
			if (disposed) return;
			mode = "source";
			modeError = `Could not load the rich text editor: ${String(error)}`;
		});
		return () => {
			disposed = true;
			modeRequest += 1;
			if (mountedEditor !== null) void mountedEditor.milkdown.destroy();
		};
	});

	async function enterRichMode() {
		if (editor === null || mode === "rich") return;
		const current = editor;
		const request = ++modeRequest;
		const source = value;
		checking = true;
		const ranges = await invoke<MarkdownSpan[]>("markdown_spans", { source }).then(
			(ranges) => ranges,
			(error) => {
				if (!disposed && value === source && request === modeRequest) {
					modeError = `Could not read Markdown spans: ${String(error)}`;
					checking = false;
				}
				return null;
			},
		);
		if (disposed || request !== modeRequest) return;
		if (value !== source || ranges === null) { checking = false; return; }
		let candidate: string;
		try { candidate = current.setMarkdown(source, ranges); }
		catch (error) {
			checking = false;
			modeError = `Could not parse Markdown in the editor: ${String(error)}`;
			return;
		}
		const equivalent = await invoke<boolean>("markdown_matches", { source, candidate }).then(
			(result) => result,
			(error) => {
				if (!disposed && value === source && request === modeRequest) modeError = `Could not verify Markdown: ${String(error)}`;
				return null;
			},
		);
		if (disposed || request !== modeRequest) return;
		checking = false;
		if (value !== source || equivalent === null) return;
		if (!equivalent) {
			modeError = "Some Markdown could not be preserved. Your complete source remains available in Markdown mode.";
			return;
		}
		modeError = "";
		mode = "rich";
	}

	function enterSourceMode() {
		modeRequest += 1;
		mode = "source";
		checking = false;
		modeError = "";
	}

	function toggleLink() {
		if (editor === null) return;
		if (toolbar.link) { editor.setLink(null); return; }
		const href = window.prompt("Link URL", "https://");
		if (href === null || href.trim() === "") return;
		if (!URL.canParse(href.trim())) { window.alert("Enter a valid HTTP or HTTPS URL."); return; }
		const url = new URL(href.trim());
		if (url.protocol !== "http:" && url.protocol !== "https:") { window.alert("Only HTTP and HTTPS links are supported."); return; }
		editor.setLink(url.href);
	}

	function insertImage() {
		if (editor === null) return;
		const source = window.prompt("Image URL", "https://");
		if (source === null || source.trim() === "") return;
		if (!URL.canParse(source.trim())) { window.alert("Enter a valid HTTP or HTTPS URL."); return; }
		const url = new URL(source.trim());
		if (url.protocol !== "http:" && url.protocol !== "https:") { window.alert("Only HTTP and HTTPS images are supported."); return; }
		const alt = window.prompt("Image description", "");
		if (alt === null) return;
		const { view } = editor;
		const image = view.state.schema.nodes.image;
		if (image) view.dispatch(view.state.tr.replaceSelectionWith(image.create({ src: url.href, alt })));
		view.focus();
	}
</script>

<div class="rich-editor">
	<div class="mode-switch" role="group" aria-label="Editor mode">
		<button type="button" class:active={mode === "rich"} disabled={editor === null || checking} aria-pressed={mode === "rich"} onclick={enterRichMode}><Pilcrow size={14} /> Rich text</button>
		<button type="button" class:active={mode === "source"} aria-pressed={mode === "source"} onclick={enterSourceMode}><FileCode2 size={14} /> Markdown</button>
	</div>
	{#if modeError}<p class="mode-error" role="status">{modeError}</p>{/if}
	<div class:hidden={mode !== "rich"} class="toolbar" onmousedown={(event) => event.preventDefault()} role="toolbar" tabindex="-1" aria-label="Article formatting">
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.heading} onclick={() => editor?.execute("heading")} aria-label="Heading" title="Heading"><Heading2 size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.bold} onclick={() => editor?.execute("bold")} aria-label="Bold" title="Bold"><Bold size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.italic} onclick={() => editor?.execute("italic")} aria-label="Italic" title="Italic"><Italic size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.strike} onclick={() => editor?.execute("strike")} aria-label="Strikethrough" title="Strikethrough"><Strikethrough size={15} /></button>
		<span></span>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.bulletList} onclick={() => editor?.execute("bulletList")} aria-label="Bullet list" title="Bullet list"><List size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.orderedList} onclick={() => editor?.execute("orderedList")} aria-label="Numbered list" title="Numbered list"><ListOrdered size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.blockquote} onclick={() => editor?.execute("blockquote")} aria-label="Quote" title="Quote"><Quote size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.codeBlock} onclick={() => editor?.execute("codeBlock")} aria-label="Code block" title="Code block"><Code size={15} /></button>
		<span></span>
		<button type="button" disabled={toolbar.sourceBlock} class:active={toolbar.link} onclick={toggleLink} aria-label={toolbar.link ? "Remove link" : "Add link"} title={toolbar.link ? "Remove link" : "Add link"}>{#if toolbar.link}<Unlink size={15} />{:else}<Link size={15} />{/if}</button>
		<button type="button" onclick={() => editor?.execute("undo")} disabled={!toolbar.canUndo} aria-label="Undo" title="Undo"><Undo2 size={15} /></button>
		<button type="button" onclick={() => editor?.execute("redo")} disabled={!toolbar.canRedo} aria-label="Redo" title="Redo"><Redo2 size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} onclick={insertImage} aria-label="Add image" title="Add image"><ImagePlus size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} onclick={() => editor?.execute("table")} aria-label="Insert table" title="Insert table"><Table size={15} /></button>
		<button type="button" disabled={toolbar.sourceBlock} onclick={() => editor?.execute("task")} aria-label="Task list" title="Task list"><ListChecks size={15} /></button>
		<small class:limit={length >= maxLength}>{length.toLocaleString()} / {maxLength.toLocaleString()}</small>
	</div>
	<div class:hidden={mode !== "rich"} class="surface" bind:this={element}></div>
	{#if mode === "source"}
		<textarea bind:value maxlength={maxLength} aria-label="Article Markdown source" spellcheck="true"></textarea>
		<small class:limit={length >= maxLength} class="source-length">{length.toLocaleString()} / {maxLength.toLocaleString()}</small>
	{/if}
</div>

<style>
	:global(.ProseMirror-selectednode) { outline: 2px solid var(--color-accent); outline-offset: 2px; }
	.rich-editor { overflow: hidden; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-background); letter-spacing: normal; text-transform: none; }
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
	.toolbar button:disabled { cursor: not-allowed; opacity: 0.35; }
	.toolbar > span { width: 1px; height: 1.25rem; margin: 0 0.2rem; background: var(--color-border); }
	.toolbar small { margin-left: auto; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.6rem; }
	.toolbar small.limit { color: var(--color-error); }
	.rich-editor > textarea { display: block; width: 100%; min-height: calc(100vh - 21rem); box-sizing: border-box; padding: 1rem 1.15rem 3rem; resize: vertical; border: 0; outline: none; background: var(--color-background); color: var(--color-foreground); font-family: var(--font-mono); font-size: 0.85rem; line-height: 1.7; }
	.source-length { display: block; padding: 0 0.75rem 0.6rem; color: var(--color-muted-foreground); font-family: var(--font-mono); font-size: 0.6rem; text-align: right; }
	.source-length.limit { color: var(--color-error); }
	.surface { min-height: calc(100vh - 21rem); max-height: calc(100vh - 14rem); overflow-y: auto; }
	.surface :global(.ProseMirror) { min-height: calc(100vh - 21rem); padding: 1rem 1.15rem 5rem; outline: none; color: var(--color-foreground); font-family: var(--font-sans); font-size: 0.9rem; line-height: 1.7; }
	.surface :global(.ProseMirror > *:first-child) { margin-top: 0; }
	.surface :global(.ProseMirror h1:not(.knowledge-prose *)), .surface :global(.ProseMirror h2:not(.knowledge-prose *)), .surface :global(.ProseMirror h3:not(.knowledge-prose *)) { margin: 1.5em 0 0.6em; }
	.surface :global(.ProseMirror h2:not(.knowledge-prose *)) { padding-bottom: 0.25em; border-bottom: 1px solid var(--color-border); }
	.surface :global(.ProseMirror p:not(.knowledge-prose *)), .surface :global(.ProseMirror ul:not(.knowledge-prose *)), .surface :global(.ProseMirror ol:not(.knowledge-prose *)), .surface :global(.ProseMirror blockquote:not(.knowledge-prose *)), .surface :global(.ProseMirror pre:not(.knowledge-prose *)) { margin: 0.8em 0; }
	.surface :global(.ProseMirror ul:not(.knowledge-prose *)), .surface :global(.ProseMirror ol:not(.knowledge-prose *)) { padding-left: 1.5rem; }
	.surface :global(.ProseMirror blockquote:not(.knowledge-prose *)) { padding-left: 1rem; border-left: 3px solid var(--color-border-strong); color: var(--color-muted-foreground); }
	.surface :global(.ProseMirror pre:not(.knowledge-prose *)) { overflow-x: auto; padding: 0.875rem 1rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-muted); font-family: var(--font-mono); }
	.surface :global(.ProseMirror code:not(.knowledge-prose *)) { font-family: var(--font-mono); }
	.surface :global(.ProseMirror a:not(.knowledge-prose *)) { color: var(--color-accent); text-decoration: underline; text-underline-offset: 0.2em; }
	.surface :global(.ProseMirror) { position: relative; white-space: pre-wrap; overflow-wrap: anywhere; }
	.surface :global(.ProseMirror h1:not(.knowledge-prose *)) { font-size: 1.65em; font-weight: 600; }
	.surface :global(.ProseMirror h2:not(.knowledge-prose *)) { font-size: 1.35em; font-weight: 600; }
	.surface :global(.ProseMirror h3:not(.knowledge-prose *)) { font-size: 1.15em; font-weight: 600; }
	.surface :global(.ProseMirror h4:not(.knowledge-prose *)), .surface :global(.ProseMirror h5:not(.knowledge-prose *)), .surface :global(.ProseMirror h6:not(.knowledge-prose *)) { font-weight: 600; }
	.surface :global(.ProseMirror ul:not(.knowledge-prose *)) { list-style-type: disc; }
	.surface :global(.ProseMirror ol:not(.knowledge-prose *)) { list-style-type: decimal; }
	.surface :global(.ProseMirror img:not(.knowledge-prose *)) { max-width: 100%; height: auto; }
	.surface :global(.ProseMirror table:not(.knowledge-prose *)) { width: 100%; border-collapse: collapse; table-layout: fixed; }
	.surface :global(.ProseMirror th:not(.knowledge-prose *)), .surface :global(.ProseMirror td:not(.knowledge-prose *)) { min-width: 3rem; padding: 0.5rem; border: 1px solid var(--color-border); vertical-align: top; }
	.surface :global(.ProseMirror th:not(.knowledge-prose *)) { background: var(--color-muted); font-weight: 600; }
	.surface :global(.ProseMirror .selectedCell) { background: color-mix(in srgb, var(--color-accent) 12%, transparent); }
	.surface :global(.ProseMirror li[data-item-type="task"]) { display: flex; align-items: baseline; gap: 0.5rem; list-style: none; }
	.surface :global(.ProseMirror li > div) { min-width: 0; flex: 1; }
	.surface :global(.ProseMirror input[type="checkbox"]) { accent-color: var(--color-accent); }
	.surface :global(.ProseMirror pre[data-markdown-source]) { white-space: pre-wrap; border-style: dashed; }
	.surface :global(.ProseMirror pre[data-markdown-source]::before) { content: "Markdown"; display: block; margin-bottom: 0.5rem; color: var(--color-muted-foreground); font-size: 0.65rem; }
	.surface :global(.ProseMirror-gapcursor) { display: none; position: absolute; pointer-events: none; }
	.surface :global(.ProseMirror-focused .ProseMirror-gapcursor) { display: block; }
	.surface :global(.ProseMirror-gapcursor::after) { content: ""; display: block; width: 1.25rem; border-top: 1px solid var(--color-foreground); }
</style>
