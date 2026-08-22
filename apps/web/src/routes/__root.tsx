// src/routes/__root.tsx
import type { ReactNode } from "react";
import {
	Outlet,
	createRootRouteWithContext,
	HeadContent,
	Scripts,
	Navigate,
} from "@tanstack/react-router";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import appCss from "@my-monorepo/ui/styles/globals.css?url";
import { getCurrentLanguage } from "@my-monorepo/i18n";
import { getThemeServerFn } from "@/lib/theme";

export interface RouterContext {
	queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
	beforeLoad: async () => ({
		theme: await getThemeServerFn(),
	}),
	head: () => ({
		meta: [
			{ charSet: "utf-8" },
			{ name: "viewport", content: "width=device-width, initial-scale=1" },
			{ title: "my-monorepo" },
		],
		links: [{ rel: "stylesheet", href: appCss }],
	}),
	component: RootComponent,
	notFoundComponent: () => <Navigate to="/{-$locale}" params={{ locale: "en" }} />,
});

function RootComponent() {
	const { queryClient } = Route.useRouteContext();
	return (
		<QueryClientProvider client={queryClient}>
			<RootDocument>
				<Outlet />
			</RootDocument>
		</QueryClientProvider>
	);
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
	const lang = getCurrentLanguage();
	const { theme } = Route.useRouteContext();
	return (
		<html lang={lang} className={theme} suppressHydrationWarning>
			<head>
				<HeadContent />
			</head>
			<body>
				{children}
				<Scripts />
			</body>
		</html>
	);
}
