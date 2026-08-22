const STORAGE_KEY = "app-theme";

export type AppTheme = "light" | "dark" | "auto";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function getTheme(): AppTheme {
  const theme = localStorage.getItem(STORAGE_KEY);
  if (theme === "light" || theme === "dark" || theme === "auto") return theme;
  return "auto";
}

export function initTheme(): void {
  const theme = getTheme();
  const resolved = theme === "auto" ? getSystemTheme() : theme;
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  root.classList.add(resolved);
  root.style.colorScheme = resolved;
}
