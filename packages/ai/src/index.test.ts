import { describe, expect, it } from "vite-plus/test";
import { createStreamId, defaultChatModel, streamChat } from "./index.js";

describe("ai package", () => {
	it("exports a default model string", () => {
		expect(typeof defaultChatModel).toBe("string");
	});

	it("exports the streamChat helper", () => {
		expect(typeof streamChat).toBe("function");
	});

	it("creates stream ids with a prefix", () => {
		const streamId = createStreamId("chat");
		expect(streamId.startsWith("chat_")).toBe(true);
	});
});
