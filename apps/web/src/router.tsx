// src/router.tsx
import { createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";
import { queryClient } from "./lib/trpc";

export function getRouter() {
	return createRouter({
		routeTree,
		scrollRestoration: true,
		context: {
			queryClient,
		},
	});
}
