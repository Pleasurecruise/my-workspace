import { mount, unmount, flushSync } from "svelte";
import { writable } from "svelte/store";
import { NodeSelection, TextSelection } from "@milkdown/kit/prose/state";
import DialectBlock from "./DialectBlock.svelte";
import {
	Editor,
	commandsCtx,
	defaultValueCtx,
	editorViewCtx,
	editorViewOptionsCtx,
	remarkStringifyOptionsCtx,
	rootCtx,
	serializerCtx,
} from "@milkdown/kit/core";
import {
	commonmark,
	imageAttr,
	imageSchema,
	listItemSchema,
	headingSchema,
	paragraphSchema,
	codeBlockSchema,
	blockquoteSchema,
	bulletListSchema,
	orderedListSchema,
	linkSchema,
	strongSchema,
	emphasisSchema,
} from "@milkdown/kit/preset/commonmark";
import { gfm, insertTableCommand, strikethroughSchema } from "@milkdown/kit/preset/gfm";
import { history } from "@milkdown/kit/plugin/history";
import { trailing } from "@milkdown/kit/plugin/trailing";
import { cursor } from "@milkdown/kit/plugin/cursor";
import { $node, $remark, $view, replaceAll } from "@milkdown/kit/utils";
import { setBlockType, toggleMark, wrapIn } from "@milkdown/kit/prose/commands";
import { liftListItem, wrapInList } from "@milkdown/kit/prose/schema-list";
import { findParentNode, getMarkRange } from "@milkdown/kit/prose";
import { redo, undo } from "@milkdown/kit/prose/history";
import type { EditorView } from "@milkdown/kit/prose/view";
import type { Root, RootContent, Literal } from "mdast";
import type { MarkdownNode } from "@milkdown/kit/transformer";
import remarkFrontmatter from "remark-frontmatter";
import type { MarkdownSpan } from "../../consumer";

declare module "mdast" {
	interface RootContentMap {
		vesperSource: Literal & { type: "vesperSource"; value: string };
	}
}

export type EditorAction =
	| "heading"
	| "bold"
	| "italic"
	| "strike"
	| "bulletList"
	| "orderedList"
	| "blockquote"
	| "codeBlock"
	| "undo"
	| "redo"
	| "table"
	| "task";

export async function createEditor(
	element: HTMLElement,
	options: {
		maxLength: number;
		onChange: (source: string) => void;
		onSelection: (view: EditorView) => void;
		onLimit: () => void;
	},
) {
	let spans: MarkdownSpan[] = [];
	let markdown = "";
	let loading = true;
	const frontmatter = $remark("vesperFrontmatter", () => remarkFrontmatter, ["yaml"]);
	const preserveSource = $remark("vesperSourceBlocks", () => () => (tree: Root, file) => {
		const source = String(file);
		// Rust identifies dialect syntax. Remark owns block boundaries; preserve the entire
		// containing block, including nested markup, rather than reinterpret its source.
		if (source !== markdown) return;
		tree.children = tree.children.flatMap((node): RootContent[] => {
			const start = node.position?.start.offset;
			const end = node.position?.end.offset;
			if (start === undefined || end === undefined) return [node];
			// Keep definitions used by literal blocks as well as by rendered reference links.
			if (node.type === "definition" && spans.length > 0)
				return [node, { type: "vesperSource", value: source.slice(start, end) }];
			if (!spans.some((range) => range.start < end && range.end > start)) return [node];
			return [{ type: "vesperSource", value: source.slice(start, end) }];
		});
	});
	const sourceSchema = $node("source_block", () => ({
		group: "block",
		content: "text*",
		marks: "",
		code: true,
		defining: true,
		atom: true,
		parseDOM: [{ tag: "pre[data-markdown-source]", preserveWhitespace: "full" }],
		toDOM: () => [
			"pre",
			{ "data-markdown-source": "", "aria-label": "Markdown source block" },
			["code", 0],
		],
		parseMarkdown: {
			match: (node) => node.type === "vesperSource",
			runner: (state, node, type) => {
				state.openNode(type);
				if (typeof node.value === "string" && node.value) state.addText(node.value);
				state.closeNode();
			},
		},
		toMarkdown: {
			match: (node) => node.type.name === "source_block",
			runner: (state, node) => {
				state.addNode("vesperSource", undefined, undefined, { value: node.textContent });
			},
		},
	}));
	const sourceView = $view(sourceSchema, () => (node, view, getPos) => {
		const dom = document.createElement("div");
		const pre = document.createElement("pre");
		pre.dataset.markdownSource = "";
		const contentDOM = document.createElement("code");
		pre.append(contentDOM);
		const source = writable(node.textContent);
		let text = node.textContent;
		const component = mount(DialectBlock, {
			target: dom,
			props: {
				source,
				content: pre,
				context: () => milkdown.ctx.get(serializerCtx)(view.state.doc),
				onEdit: (editing: boolean) => {
					const pos = getPos();
					if (pos === undefined) return;
					view.dispatch(
						view.state.tr.setSelection(
							editing
								? TextSelection.create(view.state.doc, pos + 1)
								: NodeSelection.create(view.state.doc, pos),
						),
					);
					view.focus();
				},
			},
		});
		flushSync();
		return {
			dom,
			contentDOM,
			update(current) {
				if (current.type !== node.type) return false;
				if (current.textContent !== text) {
					text = current.textContent;
					source.set(text);
				}
				return true;
			},
			stopEvent: (event) => event.target instanceof Node && !contentDOM.contains(event.target),
			ignoreMutation: (mutation) =>
				mutation.type !== "selection" && !contentDOM.contains(mutation.target),
			destroy() {
				void unmount(component);
			},
		};
	});

	const taskListView = $view(listItemSchema.node, () => (node, view, getPos) => {
		const dom = document.createElement("li");
		const contentDOM = document.createElement("div");
		const checkbox = document.createElement("input");
		checkbox.type = "checkbox";
		checkbox.contentEditable = "false";
		checkbox.setAttribute("aria-label", "Task completed");
		function updateTask(current: typeof node) {
			dom.dataset.itemType = typeof current.attrs.checked === "boolean" ? "task" : "list";
			checkbox.hidden = typeof current.attrs.checked !== "boolean";
			checkbox.checked = current.attrs.checked === true;
		}
		updateTask(node);
		checkbox.addEventListener("change", () => {
			const pos = getPos();
			if (pos === undefined) return;
			const current = view.state.doc.nodeAt(pos);
			if (current)
				view.dispatch(
					view.state.tr.setNodeMarkup(pos, undefined, {
						...current.attrs,
						checked: checkbox.checked,
					}),
				);
			checkbox.checked = view.state.doc.nodeAt(pos)?.attrs.checked === true;
		});
		dom.append(checkbox, contentDOM);
		return {
			dom,
			contentDOM,
			update(current) {
				if (current.type !== node.type) return false;
				updateTask(current);
				return true;
			},
			stopEvent: (event) => event.target === checkbox,
			ignoreMutation: (mutation) => mutation.type !== "selection" && mutation.target === checkbox,
		};
	});
	const milkdown = Editor.make()
		.config((ctx) => {
			ctx.set(rootCtx, element);
			ctx.set(defaultValueCtx, "");
			// Remark uses null for an absent image title; the image schema requires strings.
			ctx.update(imageSchema.key, (schema) => (ctx) => {
				const image = schema(ctx);
				return {
					...image,
					parseMarkdown: {
						...image.parseMarkdown,
						runner: (state, node, type) => {
							state.addNode(type, { src: node.url, alt: node.alt ?? "", title: node.title ?? "" });
						},
					},
				};
			});
			ctx.set(imageAttr.key, () => ({ loading: "lazy", referrerpolicy: "no-referrer" }));
			ctx.update(remarkStringifyOptionsCtx, (previous) => ({
				...previous,
				handlers: {
					...previous.handlers,
					vesperSource: (node: MarkdownNode) => {
						if (!("value" in node) || typeof node.value !== "string")
							throw new Error("Markdown source block has no source text");
						return node.value;
					},
				},
			}));
			ctx.update(editorViewOptionsCtx, (previous) => ({
				...previous,
				attributes: {
					"aria-label": "Article rich text",
					role: "textbox",
					"aria-multiline": "true",
				},
				handleClick: (_view, _pos, event) => {
					if (!(event.target instanceof Element) || event.target.closest("a") === null)
						return false;
					event.preventDefault();
					return true;
				},
				dispatchTransaction(transaction) {
					const view = ctx.get(editorViewCtx);
					const next = view.state.applyTransaction(transaction).state;
					const changed = !next.doc.eq(view.state.doc);
					if (changed) {
						const serialize = ctx.get(serializerCtx);
						const source = serialize(next.doc);
						const previousSource = serialize(view.state.doc);
						if (source.length > options.maxLength && source.length > previousSource.length) {
							options.onLimit();
							return;
						}
						view.updateState(next);
						// Synchronous: Save and source-mode switches must see the latest keystroke.
						if (!loading && source !== previousSource) options.onChange(source);
					} else view.updateState(next);
					options.onSelection(view);
				},
			}));
		})
		.use(frontmatter)
		.use(preserveSource)
		.use(commonmark)
		.use(gfm)
		.use(sourceSchema)
		.use(sourceView)
		.use(history)
		.use(cursor)
		.use(trailing)
		.use(taskListView);
	await milkdown.create();
	loading = false;
	return {
		milkdown,
		view: milkdown.ctx.get(editorViewCtx),
		setMarkdown(source: string, ranges: MarkdownSpan[]) {
			markdown = source;
			spans = ranges;
			loading = true;
			try {
				milkdown.action(replaceAll(source, true));
			} finally {
				loading = false;
			}
			const view = milkdown.ctx.get(editorViewCtx);
			options.onSelection(view);
			return milkdown.ctx.get(serializerCtx)(view.state.doc);
		},
		execute(action: EditorAction) {
			const view = milkdown.ctx.get(editorViewCtx);
			const { state, dispatch } = view;
			const ctx = milkdown.ctx;
			if (action === "undo") undo(state, dispatch);
			else if (action === "redo") redo(state, dispatch);
			else if (!readSelection(view).sourceBlock) {
				if (action === "table") milkdown.ctx.get(commandsCtx).call(insertTableCommand.key);
				else if (action === "bold" || action === "italic" || action === "strike")
					toggleMark(
						{ bold: strongSchema, italic: emphasisSchema, strike: strikethroughSchema }[
							action
						].type(ctx),
					)(state, dispatch);
				else if (action === "heading" || action === "codeBlock") {
					const name = action === "heading" ? "heading" : "code_block";
					setBlockType(
						state.selection.$from.parent.type.name === name
							? paragraphSchema.type(ctx)
							: (action === "heading" ? headingSchema : codeBlockSchema).type(ctx),
						action === "heading" ? { level: 2 } : {},
					)(state, dispatch);
				} else if (action === "blockquote") {
					const quote = findParentNode((node) => node.type === blockquoteSchema.type(ctx))(
						state.selection,
					);
					if (quote)
						dispatch(
							state.tr.replaceWith(quote.pos, quote.pos + quote.node.nodeSize, quote.node.content),
						);
					else wrapIn(blockquoteSchema.type(ctx))(state, dispatch);
				} else {
					const list = (action === "orderedList" ? orderedListSchema : bulletListSchema).type(ctx);
					const parent = findParentNode(
						(node) =>
							node.type === bulletListSchema.type(ctx) || node.type === orderedListSchema.type(ctx),
					)(state.selection);
					if (!parent) wrapInList(list)(state, dispatch);
					else if (action !== "task") {
						if (parent.node.type === list) liftListItem(listItemSchema.type(ctx))(state, dispatch);
						else {
							const transaction = state.tr.setNodeMarkup(parent.pos, list, parent.node.attrs);
							parent.node.forEach((item, offset, index) => {
								transaction.setNodeMarkup(parent.pos + 1 + offset, undefined, {
									...item.attrs,
									listType: action === "orderedList" ? "ordered" : "bullet",
									label: action === "orderedList" ? `${index + 1}.` : "•",
								});
							});
							dispatch(transaction);
						}
					}
					if (action === "task") {
						const current = view.state;
						const item = findParentNode((node) => node.type === listItemSchema.type(ctx))(
							current.selection,
						);
						if (item)
							view.dispatch(
								current.tr.setNodeMarkup(item.pos, undefined, {
									...item.node.attrs,
									checked: typeof item.node.attrs.checked === "boolean" ? null : false,
								}),
							);
					}
				}
			}
			view.focus();
		},
		setLink(href: string | null) {
			const view = milkdown.ctx.get(editorViewCtx);
			const { state, dispatch } = view;
			const { from, to, $from, empty } = state.selection;
			const type = linkSchema.type(milkdown.ctx);
			if (href !== null) toggleMark(type, { href })(state, dispatch);
			else if (!empty) dispatch(state.tr.removeMark(from, to, type));
			else {
				const range = getMarkRange($from, type);
				if (range) dispatch(state.tr.removeMark(range.from, range.to, type).removeStoredMark(type));
			}
			view.focus();
		},
	};
}

export function readSelection(view: EditorView) {
	const { selection, storedMarks, doc, schema } = view.state;
	const { from, to, $from, empty } = selection;
	const ancestors = Array.from(
		{ length: $from.depth },
		(_, index) => $from.node(index + 1).type.name,
	);
	let sourceBlock = ancestors.includes("source_block");
	doc.nodesBetween(from, to, (node) => {
		if (node.type.name === "source_block") sourceBlock = true;
	});
	const hasMark = (name: string) => {
		const type = schema.marks[name];
		return (
			type !== undefined &&
			(empty
				? Boolean(type.isInSet(storedMarks || $from.marks()))
				: doc.rangeHasMark(from, to, type))
		);
	};
	return {
		heading: $from.parent.type.name === "heading" && $from.parent.attrs.level === 2,
		bold: hasMark("strong"),
		italic: hasMark("emphasis"),
		strike: hasMark("strike_through"),
		link: hasMark("link"),
		bulletList: ancestors.includes("bullet_list"),
		orderedList: ancestors.includes("ordered_list"),
		blockquote: ancestors.includes("blockquote"),
		codeBlock: ancestors.includes("code_block"),
		sourceBlock,
		canUndo: undo(view.state),
		canRedo: redo(view.state),
	};
}
