import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, KnowledgeDocument } from "../consumer";
import KnowledgeView from "./KnowledgeView.svelte";

it("restores an article draft after navigation and preserves changes made during a save", async () => {
  let resolvePending!: (response: CommandResponse<KnowledgeDocument>) => void;
  const pending = new Promise<CommandResponse<KnowledgeDocument>>((resolve) => {
    resolvePending = resolve;
  });
  const props = {
    documents: [],
    loading: false,
    oncreate: vi.fn(() => pending),
    onupdate: vi.fn(),
  };
  const target = document.createElement("div");
  document.body.append(target);
  let view = mount(KnowledgeView, { target, props });
  await tick();
  target.querySelector<HTMLButtonElement>(".index-header button")!.click();
  await tick();
  const title = target.querySelector<HTMLInputElement>('[placeholder="Untitled knowledge"]')!;
  const summary = target.querySelector<HTMLInputElement>('[placeholder="A short summary"]')!;
  title.value = "Draft title";
  title.dispatchEvent(new Event("input", { bubbles: true }));
  summary.value = "Draft summary";
  summary.dispatchEvent(new Event("input", { bubbles: true }));
  target.querySelector<HTMLButtonElement>(".mode-switch button:last-child")!.click();
  await tick();
  const body = target.querySelector<HTMLTextAreaElement>('[aria-label="Article Markdown source"]')!;
  body.value = "Draft body";
  body.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  target.querySelector<HTMLButtonElement>(".editor-actions .save")!.click();
  expect(props.oncreate).toHaveBeenCalledWith({
    title: "Draft title",
    summary: "Draft summary",
    body: "Draft body",
    tags: [],
  });
  title.value = "Revised title";
  title.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  await unmount(view);
  view = mount(KnowledgeView, { target, props });
  await tick();
  expect(target.querySelector<HTMLInputElement>('[placeholder="Untitled knowledge"]')!.value).toBe(
    "Revised title",
  );
  resolvePending({
    status: "ready",
    data: {
      id: "article-1",
      slug: "article-1",
      title: "Draft title",
      summary: "Draft summary",
      tags: [],
      visibility: "private",
      contentHash: "saved-hash",
      createdAt: "2026-09-05",
      updatedAt: "2026-09-05",
      newspaperEdition: null,
      source: "Draft body",
      html: "<p>Draft body</p>",
      toc: [],
    },
  });
  await vi.waitFor(() =>
    expect(target.querySelector<HTMLButtonElement>(".editor-actions .save")!.disabled).toBe(false),
  );
  expect(target.querySelector<HTMLInputElement>('[placeholder="Untitled knowledge"]')!.value).toBe(
    "Revised title",
  );
  expect(target.querySelector(".reader")).toBeNull();
  await unmount(view);
  target.remove();
});
