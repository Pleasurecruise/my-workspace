import { defineConfig } from "vite-plus";

export default defineConfig({
	fmt: {
		overrides: [{ files: ["**/*.{js,ts}"], options: { useTabs: true } }],
	},
	test: {
		projects: ["apps/desktop/vite.config.ts"],
		coverage: {
			reportsDirectory: "coverage/frontend",
			include: ["apps/desktop/src/**/*.{ts,svelte}"],
			exclude: ["**/__tests__/**"],
			reporter: ["text", "html", "json-summary"],
		},
	},
});
