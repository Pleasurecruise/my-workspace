import { describe, expect, it } from "vite-plus/test";
import { appName, foundations } from "./lib/app";

describe("desktop foundation", () => {
  it("keeps the application identity and runtime boundaries explicit", () => {
    expect(appName).toBe("Workspace CMS");
    expect(foundations.map(({ value }) => value)).toEqual(["Svelte 5", "Tauri 2", "Rust"]);
  });
});
