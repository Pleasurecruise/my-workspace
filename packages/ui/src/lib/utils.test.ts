import { describe, expect, it } from "vite-plus/test";
import { cn } from "./utils";

describe("utils", () => {
  it("merges conflicting utility classes", () => {
    expect(cn("px-2", false, "px-4")).toBe("px-4");
  });
});
