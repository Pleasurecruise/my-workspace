import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { queryClient } from "@/lib/trpc";
import "@my-monorepo/ui/styles/globals.css";
import { initI18n } from "@/lib/i18n";
import { initTheme } from "@/lib/theme";
import App from "@/App";

initI18n();
initTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<App />
		</QueryClientProvider>
	</React.StrictMode>,
);
