import { describe, expect, it } from "vite-plus/test";
import { prisma } from "./index.js";

describe("db exports", () => {
	it("should export prisma client", () => {
		expect(prisma).toBeDefined();
	});
});
