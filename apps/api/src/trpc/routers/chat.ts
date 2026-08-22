import { isAiConfigured, streamChat, type ModelMessage } from "@my-monorepo/ai";
import { createLoggerWithContext } from "@my-monorepo/logger";
import { z } from "@my-monorepo/utils";
import { TRPCError } from "@trpc/server";
import { createTRPCRouter, publicProcedure } from "../init";

type MessageStatus = "streaming" | "done" | "error";

type StreamChunk = {
	status: MessageStatus;
	text: string;
};

const logger = createLoggerWithContext("api:chat");

const chatMessageSchema = z.object({
	role: z.enum(["user", "assistant", "system"]),
	content: z.string(),
});

export const chatRouter = createTRPCRouter({
	// TODO: Replace with protectedProcedure once auth is implemented.
	sendMessage: publicProcedure
		.input(
			z.object({
				messages: z.array(chatMessageSchema).min(1),
			}),
		)
		.mutation(async function* ({ input }): AsyncGenerator<StreamChunk> {
			if (!isAiConfigured()) {
				throw new TRPCError({
					code: "INTERNAL_SERVER_ERROR",
					message: "服务器配置错误：缺少AI配置",
				});
			}

			try {
				const { textStream } = streamChat({
					messages: input.messages as ModelMessage[],
				});

				for await (const text of textStream) {
					yield { status: "streaming", text };
				}

				yield { status: "done", text: "" };
			} catch (error) {
				logger.error("AI stream error", error instanceof Error ? { error } : { error });
				throw new TRPCError({
					code: "INTERNAL_SERVER_ERROR",
					message: "AI服务出现错误，请稍后再试",
				});
			}
		}),
});
