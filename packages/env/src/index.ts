import { createEnv } from "@t3-oss/env-core";
import {
	aiEnvSchema,
	authEnvSchema,
	cryptoEnvSchema,
	databaseEnvSchema,
	logEnvSchema,
	mailEnvSchema,
	publicEnvSchema,
} from "./schema.js";

export const env = createEnv({
	server: {
		...publicEnvSchema.shape,
		...logEnvSchema.shape,
		...databaseEnvSchema.shape,
		...authEnvSchema.shape,
		...aiEnvSchema.shape,
		...mailEnvSchema.shape,
		...cryptoEnvSchema.shape,
	},
	runtimeEnv: process.env,
	emptyStringAsUndefined: true,
	isServer: true,
	skipValidation: !!process.env.VITEST,
});

export type {
	AiEnv,
	AuthEnv,
	CryptoEnv,
	DatabaseEnv,
	Env,
	LogEnv,
	MailEnv,
	PublicEnv,
} from "./schema.js";
