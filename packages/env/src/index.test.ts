import { describe, expect, it } from "vite-plus/test";
import { clientSchema, serverSchema as schema } from "./schema.js";

describe("env schema", () => {
	it("should validate a valid environment", () => {
		const validEnv = {
			DATABASE_URL: "postgresql://localhost:5432/db",
			BETTER_AUTH_SECRET: "secret",
		};

		const result = schema.safeParse(validEnv);
		expect(result.success).toBe(true);
	});

	it("should fail on invalid environment", () => {
		const invalidEnv = {
			DATABASE_URL: "not-a-url",
		};

		const result = schema.safeParse(invalidEnv);
		expect(result.success).toBe(false);
	});

	it("should use default values", () => {
		const minimalEnv = {
			DATABASE_URL: "postgresql://localhost:5432/db",
			BETTER_AUTH_SECRET: "secret",
		};

		const result = schema.parse(minimalEnv);
		expect(result.LOG_LEVEL).toBe("info");
		expect(result.PUBLIC_API_ORIGIN).toBe("http://localhost:5173");
		expect(result.PUBLIC_WEB_ORIGIN).toBe("http://localhost:3000");
	});

	it("should coerce boolean values from runtime strings", () => {
		const result = schema.parse({
			DATABASE_URL: "postgresql://localhost:5432/db",
			BETTER_AUTH_SECRET: "secret",
			LOG_PRETTY: "true",
			MAIL_SECURE: "false",
		});

		expect(result.LOG_PRETTY).toBe(true);
		expect(result.MAIL_SECURE).toBe(false);
	});
});

describe("client env schema", () => {
	it("should only include public client variables", () => {
		const result = clientSchema.parse({
			PUBLIC_API_ORIGIN: "http://localhost:5173",
			PUBLIC_WEB_ORIGIN: "http://localhost:3000",
		});

		expect(result.PUBLIC_API_ORIGIN).toBe("http://localhost:5173");
		expect(result.PUBLIC_WEB_ORIGIN).toBe("http://localhost:3000");
	});
});
