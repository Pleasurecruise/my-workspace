import { afterEach, beforeEach, expect, it, vi } from "vite-plus/test";

const { setTheme } = vi.hoisted(() => ({ setTheme: vi.fn(async () => undefined) }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ setTheme }) }));

import { applyTheme, initTheme } from "../theme";

beforeEach(() => {
	localStorage.clear();
	setTheme.mockClear();
});

afterEach(() => {
	vi.unstubAllGlobals();
});

it("follows the system preference until a theme is saved", () => {
	vi.stubGlobal("matchMedia", () => ({ matches: true }));
	expect(initTheme()).toBe(true);
	expect(document.documentElement.classList.contains("dark")).toBe(true);
	expect(localStorage.getItem("app-theme")).toBe("dark");

	localStorage.setItem("app-theme", "light");
	expect(initTheme()).toBe(false);
	expect(document.documentElement.classList.contains("light")).toBe(true);
	expect(document.documentElement.style.colorScheme).toBe("light");
});

it("persists explicit choices and synchronizes the native window", () => {
	vi.stubGlobal("matchMedia", () => ({ matches: false }));
	applyTheme(true);
	expect(document.documentElement.classList.contains("dark")).toBe(true);
	expect(document.documentElement.classList.contains("light")).toBe(false);
	expect(setTheme).toHaveBeenLastCalledWith("dark");
	expect(initTheme()).toBe(true);
});
