import type { AsyncIterableStream, ModelMessage } from "ai";

export type { ModelMessage };

export type StreamChatOptions = {
	messages: ModelMessage[];
	model?: string;
	temperature?: number;
	maxOutputTokens?: number;
};

export type StreamChatResult = {
	textStream: AsyncIterableStream<string>;
};
