import { describe, expect, it } from "vite-plus/test";
import { createPrefixedId } from "./ids.js";

describe("ids", () => {
	it("should create a prefixed id", () => {
		const id = createPrefixedId("test");
		expect(id).toMatch(/^test_/);
		expect(id.length).toBeGreaterThan(5);
	});

	it("should create a default prefixed id", () => {
		const id = createPrefixedId();
		expect(id).toMatch(/^id_/);
	});

	it("should create different ids", () => {
		const id1 = createPrefixedId("test");
		const id2 = createPrefixedId("test");
		expect(id1).not.toBe(id2);
	});
});
