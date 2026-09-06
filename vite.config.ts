import { defineConfig } from "vite-plus";

export default defineConfig({
	fmt: {
		overrides: [{ files: ["**/*.ts"], options: { useTabs: true } }],
	},
	test: {
		projects: ["apps/desktop/vite.config.ts"],
	},
	run: {
		cache: true,
	},
});
