import { env } from "@my-monorepo/env";
import { defineConfig } from "prisma/config";

export default defineConfig({
	schema: "prisma/schema.prisma",
	datasource: {
		url: env.DATABASE_URL,
	},
	migrations: {
		path: "prisma/migrations",
	},
});
