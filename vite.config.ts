import { defineConfig } from "vite-plus";

export default defineConfig({
	fmt: {
		overrides: [{ files: ["**/*.ts"], options: { useTabs: true } }],
	},
	test: {
		projects: ["apps/desktop/vite.config.ts"],
		coverage: {
			include: ["apps/desktop/src/**/*.{ts,svelte}"],
			exclude: ["**/__tests__/**"],
			reporter: ["text", "html", "json-summary"],
		},
	},
	run: {
		cache: true,
	},
});
