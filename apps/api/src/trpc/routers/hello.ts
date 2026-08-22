import { createTRPCRouter, publicProcedure } from "../init";

export const helloRouter = createTRPCRouter({
	greet: publicProcedure.query(() => {
		return { message: "Hello from tRPC!" };
	}),
});
