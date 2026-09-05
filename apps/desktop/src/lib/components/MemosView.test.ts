import { expect, it, vi } from "vite-plus/test";
import { mount, tick, unmount } from "svelte";
import type { CommandResponse, MemoView } from "../consumer";
import MemosView from "./MemosView.svelte";

it("retains a composer draft across navigation and keeps edits made during a save", async () => {
  let resolvePending!: (response: CommandResponse<MemoView>) => void;
  const pending = new Promise<CommandResponse<MemoView>>((resolve) => {
    resolvePending = resolve;
  });
  const props = {
    memos: [],
    tags: [],
    display: "active" as const,
    onfilter: vi.fn().mockResolvedValue(null),
    onopenmemo: vi.fn(),
    oncreate: vi.fn(() => pending),
    onimportx: vi.fn(),
    onupdate: vi.fn(),
    ondelete: vi.fn(),
    onpublishtelegram: vi.fn(),
    onpublishx: vi.fn(),
  };
  const target = document.createElement("div");
  document.body.append(target);
  let view = mount(MemosView, { target, props });
  await tick();
  let input = target.querySelector<HTMLTextAreaElement>("textarea")!;
  input.value = "Submitted draft";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  target.querySelector<HTMLButtonElement>(".composer-toolbar button:last-child")!.click();
  expect(props.oncreate).toHaveBeenCalledWith("Submitted draft", "private");
  input.value = "Draft with later edits";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  await unmount(view);
  view = mount(MemosView, { target, props });
  await tick();
  input = target.querySelector<HTMLTextAreaElement>("textarea")!;
  expect(input.value).toBe("Draft with later edits");
  expect(
    target.querySelector<HTMLButtonElement>(".composer-toolbar button:last-child")!.disabled,
  ).toBe(true);
  resolvePending({
    status: "ready",
    data: {
      id: "memo-1",
      r2Key: "memo-1.md",
      content: "Submitted draft",
      html: "<p>Submitted draft</p>",
      tags: [],
      createdAt: "2026-09-05",
      updatedAt: "2026-09-05",
      visibility: "private",
      pinned: false,
      favorite: false,
      archived: false,
      metadataComplete: true,
    },
  });
  await vi.waitFor(() =>
    expect(
      target.querySelector<HTMLButtonElement>(".composer-toolbar button:last-child")!.disabled,
    ).toBe(false),
  );
  expect(input.value).toBe("Draft with later edits");
  await unmount(view);
  target.remove();
});

it("preserves public edit state across navigation and retains a failed edit for retry", async () => {
  const memo: MemoView = {
    id: "public-memo",
    r2Key: "public-memo.md",
    content: "Original",
    html: "<p>Original</p>",
    tags: [],
    createdAt: "2026-09-05",
    updatedAt: "2026-09-05",
    visibility: "public",
    pinned: false,
    favorite: false,
    archived: false,
    metadataComplete: true,
  };
  const props = {
    memos: [memo],
    tags: [],
    display: "active" as const,
    onfilter: vi.fn().mockResolvedValue(null),
    onopenmemo: vi.fn(),
    oncreate: vi.fn(),
    onimportx: vi.fn(),
    onupdate: vi
      .fn()
      .mockResolvedValueOnce({ status: "failed", message: "Save unavailable" })
      .mockResolvedValueOnce({ status: "ready", data: { ...memo, content: "Revised" } }),
    ondelete: vi.fn(),
    onpublishtelegram: vi.fn(),
    onpublishx: vi.fn(),
  };
  const target = document.createElement("div");
  document.body.append(target);
  let view = mount(MemosView, { target, props });
  await tick();
  Array.from(target.querySelectorAll<HTMLButtonElement>("article footer button"))
    .find((button) => button.textContent?.trim() === "Edit")!
    .click();
  await tick();
  await unmount(view);
  view = mount(MemosView, { target, props });
  await tick();
  expect(target.querySelector<HTMLTextAreaElement>(".inline-editor textarea")!.value).toBe(
    "Original",
  );
  target.querySelector<HTMLButtonElement>("article footer button:last-child")!.click();
  await tick();
  expect(props.onupdate).not.toHaveBeenCalled();
  expect(target.querySelector(".inline-editor")).toBeNull();
  Array.from(target.querySelectorAll<HTMLButtonElement>("article footer button"))
    .find((button) => button.textContent?.trim() === "Edit")!
    .click();
  await tick();
  const input = target.querySelector<HTMLTextAreaElement>(".inline-editor textarea")!;
  input.value = "Revised";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  target.querySelector<HTMLButtonElement>("article footer button:last-child")!.click();
  await vi.waitFor(() => expect(target.textContent).toContain("Save unavailable"));
  await unmount(view);
  view = mount(MemosView, { target, props });
  await tick();
  expect(target.querySelector<HTMLTextAreaElement>(".inline-editor textarea")!.value).toBe(
    "Revised",
  );
  target.querySelector<HTMLButtonElement>("article footer button:last-child")!.click();
  await vi.waitFor(() => expect(target.querySelector(".inline-editor")).toBeNull());
  expect(props.onupdate).toHaveBeenCalledTimes(2);
  expect(props.onupdate).toHaveBeenLastCalledWith("public-memo", {
    content: "Revised",
    visibility: "public",
  });
  await unmount(view);
  target.remove();
});
