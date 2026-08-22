import { z } from "zod";

const booleanFromRuntime = z.preprocess((value) => {
	if (typeof value === "boolean") {
		return value;
	}

	if (typeof value === "string") {
		const normalized = value.trim().toLowerCase();
		if (normalized === "true") {
			return true;
		}
		if (normalized === "false") {
			return false;
		}
	}

	return value;
}, z.boolean());

export const publicEnvSchema = z.object({
	PUBLIC_API_ORIGIN: z.string().url().default("http://localhost:5173"),
	PUBLIC_WEB_ORIGIN: z.string().url().default("http://localhost:3000"),
	PUBLIC_TAURI_ORIGIN: z.string().url().default("http://localhost:1420"),
});

export const logEnvSchema = z.object({
	LOG_LEVEL: z.enum(["trace", "debug", "info", "warn", "error", "fatal"]).default("info"),
	LOG_PRETTY: booleanFromRuntime.default(false),
});

export const databaseEnvSchema = z.object({
	DATABASE_URL: z.string().url(),
});

export const authEnvSchema = z.object({
	BETTER_AUTH_SECRET: z.string().min(1),
	GITHUB_CLIENT_ID: z.string().optional(),
	GITHUB_CLIENT_SECRET: z.string().optional(),
	GOOGLE_CLIENT_ID: z.string().optional(),
	GOOGLE_CLIENT_SECRET: z.string().optional(),
});

export const aiEnvSchema = z.object({
	OPENAI_API_KEY: z.string().optional(),
	OPENAI_API_URL: z.string().url().default("https://api.siliconflow.cn"),
	OPENAI_MODEL: z.string().default("deepseek-ai/DeepSeek-V3"),
});

export const mailEnvSchema = z.object({
	MAIL_HOST: z.string().optional(),
	MAIL_PORT: z.coerce.number().default(587),
	MAIL_SECURE: booleanFromRuntime.default(false),
	MAIL_AUTH_USER: z.string().optional(),
	MAIL_AUTH_PASS: z.string().optional(),
	MAIL_FROM: z.string().optional(),
});

export const cryptoEnvSchema = z.object({
	ENCRYPTION_KEY: z.string().length(64).optional(),
	FILE_KEY_SECRET: z.string().optional(),
});

export const serverSchema = publicEnvSchema
	.merge(logEnvSchema)
	.merge(databaseEnvSchema)
	.merge(authEnvSchema)
	.merge(aiEnvSchema)
	.merge(mailEnvSchema)
	.merge(cryptoEnvSchema);

export const clientSchema = publicEnvSchema;

export type PublicEnv = z.infer<typeof publicEnvSchema>;
export type LogEnv = z.infer<typeof logEnvSchema>;
export type DatabaseEnv = z.infer<typeof databaseEnvSchema>;
export type AuthEnv = z.infer<typeof authEnvSchema>;
export type AiEnv = z.infer<typeof aiEnvSchema>;
export type MailEnv = z.infer<typeof mailEnvSchema>;
export type CryptoEnv = z.infer<typeof cryptoEnvSchema>;
export type Env = z.infer<typeof serverSchema>;
export type ClientEnv = z.infer<typeof clientSchema>;
