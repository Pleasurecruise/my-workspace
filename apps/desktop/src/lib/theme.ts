import { getCurrentWindow } from "@tauri-apps/api/window";

const STORAGE_KEY = "app-theme";
const appWindow = getCurrentWindow();

export function initTheme(): boolean {
	const saved = localStorage.getItem(STORAGE_KEY);
	const dark =
		saved === "dark" ||
		(saved !== "light" && window.matchMedia("(prefers-color-scheme: dark)").matches);
	applyTheme(dark);
	return dark;
}

export function applyTheme(dark: boolean): void {
	const root = document.documentElement;
	root.classList.toggle("dark", dark);
	root.classList.toggle("light", !dark);
	root.style.colorScheme = dark ? "dark" : "light";
	localStorage.setItem(STORAGE_KEY, dark ? "dark" : "light");
	void appWindow.setTheme(dark ? "dark" : "light");
}
