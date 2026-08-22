import { describe, it, expect } from "vite-plus/test";
import { appRouter } from "./trpc/routers";

describe("tRPC Router", () => {
	it("hello.greet returns message", async () => {
		const caller = appRouter.createCaller({ session: null });
		const result = await caller.hello.greet();
		expect(result).toEqual({ message: "Hello from tRPC!" });
	});
});
