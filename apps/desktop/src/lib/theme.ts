import { getCurrentWindow } from "@tauri-apps/api/window";

const STORAGE_KEY = "app-theme";
const appWindow = getCurrentWindow();

export type AppTheme = "light" | "dark" | "auto";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function getTheme(): AppTheme {
  const theme = localStorage.getItem(STORAGE_KEY);
  if (theme === "light" || theme === "dark" || theme === "auto") return theme;
  return "auto";
}

export function initTheme(): boolean {
  const theme = getTheme();
  const resolved = theme === "auto" ? getSystemTheme() : theme;
  applyTheme(resolved === "dark");
  return resolved === "dark";
}

export function applyTheme(dark: boolean): void {
  const root = document.documentElement;
  root.classList.toggle("dark", dark);
  root.classList.toggle("light", !dark);
  root.style.colorScheme = dark ? "dark" : "light";
  localStorage.setItem(STORAGE_KEY, dark ? "dark" : "light");
  void appWindow.setTheme(dark ? "dark" : "light");
}
