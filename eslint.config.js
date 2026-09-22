import js from "@eslint/js";
import { plugin as shadcn } from "@shadcn/lint";
import { defineConfig, globalIgnores } from "eslint/config";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import ts from "typescript-eslint";
import desktopSvelteConfig from "./apps/desktop/svelte.config.js";
import uiSvelteConfig from "./packages/ui/svelte.config.js";

export default defineConfig(
	globalIgnores([
		"**/dist/**",
		"**/target/**",
		"**/coverage/**",
		"**/.vite/**",
		"**/.vitest/**",
		"**/.svelte-check/**",
		"**/.svelte-kit/**",
		"**/.void/**",
		".agents/**",
		".codex/**",
		"**/src-tauri/gen/**",
	]),
	js.configs.recommended,
	ts.configs.recommended,
	svelte.configs.recommended,
	{
		linterOptions: { noInlineConfig: true },
	},
	{
		files: ["apps/desktop/src/**/*.{js,ts,svelte}", "packages/ui/src/**/*.{js,ts,svelte}"],
		ignores: ["apps/desktop/src/__tests__/lint.test.ts"],
		languageOptions: { globals: globals.browser },
	},
	{
		files: ["**/*.config.{js,ts}", "**/*.test.ts"],
		languageOptions: { globals: globals.nodeBuiltin },
	},
	{
		// These views render only HTML compiled by the Rust Markdown boundary.
		files: [
			"apps/desktop/src/lib/components/pages/KnowledgeView.svelte",
			"apps/desktop/src/lib/components/pages/MemosView.svelte",
			"apps/desktop/src/lib/components/pages/NewspaperView.svelte",
		],
		rules: { "svelte/no-at-html-tags": "off" },
	},
	{
		// Core ESLint cannot see the parent reading this component's bindable output.
		files: ["apps/desktop/src/lib/components/pages/MusicView.svelte"],
		rules: { "no-useless-assignment": "off" },
	},
	{
		files: ["**/*.test.ts"],
		rules: {
			"@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
		},
	},
	{
		files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
		languageOptions: { parserOptions: { parser: ts.parser } },
		rules: { "svelte/comment-directive": "off" },
	},
	{
		files: ["apps/desktop/**/*.svelte"],
		languageOptions: { parserOptions: { svelteConfig: desktopSvelteConfig } },
	},
	{
		files: ["packages/ui/**/*.svelte"],
		languageOptions: { parserOptions: { svelteConfig: uiSvelteConfig } },
	},
	{
		files: ["**/*.ts", "**/*.svelte"],
		plugins: { shadcn },
		rules: { "shadcn/no-arbitrary-values": "error" },
	},
);
