import { defineConfig } from "vite-plus";

export default defineConfig({
	lint: {
		plugins: ["react", "typescript", "import"],
		env: {
			browser: true,
			es2024: true,
			node: true,
		},
		rules: {
			"no-unused-vars": "warn",
			"no-console": "warn",
			eqeqeq: "warn",
			"react-hooks/rules-of-hooks": "error",
			"react-hooks/exhaustive-deps": "warn",
		},
		ignorePatterns: [
			"node_modules",
			"dist",
			"build",
			".next",
			"**/generated/**",
			"*.gen.ts",
			"*.gen.js",
		],
	},
	fmt: {
		useTabs: true,
		singleQuote: false,
		experimentalSortPackageJson: false,
		ignorePatterns: ["**/generated/**", "**/routeTree.gen.ts", "**/dist/**"],
	},
});
