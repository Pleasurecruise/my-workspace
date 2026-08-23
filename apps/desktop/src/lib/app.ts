export const appName = "Workspace CMS";

type Foundation = {
  label: string;
  value: string;
};

export const foundations: Foundation[] = [
  { label: "Interface", value: "Svelte 5" },
  { label: "Runtime", value: "Tauri 2" },
  { label: "CMS Core", value: "Rust" },
];
