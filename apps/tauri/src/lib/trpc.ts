import { createTRPCClient, httpBatchLink } from "@trpc/client";
import { createTRPCOptionsProxy } from "@trpc/tanstack-react-query";
import type { AppRouter } from "@my-monorepo/api/routers";
import { env } from "@my-monorepo/env/client";
import { superjson } from "@my-monorepo/utils";
import { createQueryClient } from "@/lib/query-client";

const queryClient = createQueryClient();

const trpcClient = createTRPCClient<AppRouter>({
	links: [
		httpBatchLink({
			url: `${env.PUBLIC_API_ORIGIN}/trpc`,
			transformer: superjson,
		}),
	],
});

export const trpc = createTRPCOptionsProxy<AppRouter>({
	client: trpcClient,
	queryClient,
});

export { queryClient };
