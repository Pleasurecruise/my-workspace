import { defineConfig } from "vite-plus";

export default defineConfig({
  test: {
    projects: [
      { test: { name: "ui", include: ["packages/ui/**/*.test.ts"] } },
      "apps/desktop/vite.config.ts",
    ],
  },
  run: {
    cache: true,
  },
});
