import type { inferRouterInputs, inferRouterOutputs } from "@trpc/server";
import { createTRPCRouter } from "../init";
import { chatRouter } from "./chat";
import { helloRouter } from "./hello";

export const appRouter = createTRPCRouter({
	chat: chatRouter,
	hello: helloRouter,
});

export type AppRouter = typeof appRouter;
export type RouterOutputs = inferRouterOutputs<AppRouter>;
export type RouterInputs = inferRouterInputs<AppRouter>;
