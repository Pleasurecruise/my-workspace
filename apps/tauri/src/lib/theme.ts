const STORAGE_KEY = "app-theme";

export type AppTheme = "light" | "dark" | "auto";

function getSystemTheme(): "light" | "dark" {
	return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(theme: AppTheme): void {
	const resolved = theme === "auto" ? getSystemTheme() : theme;
	const root = document.documentElement;
	root.classList.remove("light", "dark");
	root.classList.add(resolved);
	root.style.colorScheme = resolved;

	// Tauri v2: pass null for "auto" so the native window follows the system
	import("@tauri-apps/api/window")
		.then(({ getCurrentWindow }) => {
			return getCurrentWindow().setTheme(theme === "auto" ? null : resolved);
		})
		.catch(() => {});
}

export function getTheme(): AppTheme {
	return (localStorage.getItem(STORAGE_KEY) as AppTheme) ?? "auto";
}

export function setTheme(theme: AppTheme): void {
	localStorage.setItem(STORAGE_KEY, theme);
	applyTheme(theme);
}

export function initTheme(): void {
	applyTheme(getTheme());
}
