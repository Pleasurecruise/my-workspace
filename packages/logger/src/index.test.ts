import { describe, expect, it } from "vite-plus/test";
import { createLoggerWithContext, logger } from "./index.js";

describe("logger", () => {
	it("exposes standard log methods", () => {
		expect(logger).toMatchObject({
			info: expect.any(Function),
			error: expect.any(Function),
			warn: expect.any(Function),
			debug: expect.any(Function),
			trace: expect.any(Function),
			fatal: expect.any(Function),
		});
	});

	it("creates a child logger with context", () => {
		const child = createLoggerWithContext("test");
		expect(child).toMatchObject({
			info: expect.any(Function),
			error: expect.any(Function),
			warn: expect.any(Function),
			debug: expect.any(Function),
			trace: expect.any(Function),
			fatal: expect.any(Function),
		});
	});
});
