import { TRPCError, initTRPC } from "@trpc/server";
import { superjson } from "@my-monorepo/utils";

type Session = {
	userId: string;
};

type TRPCContext = {
	session: Session | null;
};

export const createTRPCContext = async (): Promise<TRPCContext> => {
	// TODO: Implement actual session verification
	return {
		session: null,
	};
};

const t = initTRPC.context<TRPCContext>().create({
	transformer: superjson,
});

export const createTRPCRouter = t.router;
export const createCallerFactory = t.createCallerFactory;

export const publicProcedure = t.procedure;

export const protectedProcedure = t.procedure.use(async (opts) => {
	const { session } = opts.ctx;

	if (!session) {
		throw new TRPCError({ code: "UNAUTHORIZED" });
	}

	return opts.next({
		ctx: {
			session,
		},
	});
});
