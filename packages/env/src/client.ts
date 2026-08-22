import { createEnv } from "@t3-oss/env-core";
import { publicEnvSchema } from "./schema.js";

export const env = createEnv({
	clientPrefix: "PUBLIC_",
	client: {
		...publicEnvSchema.shape,
	},
	runtimeEnv: (import.meta as ImportMeta & { env: Record<string, string | undefined> }).env,
	emptyStringAsUndefined: true,
});

export type { ClientEnv as Env } from "./schema.js";
