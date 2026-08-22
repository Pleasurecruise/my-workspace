import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { createBetterAuthClient } from "@my-monorepo/auth/client";
import { env } from "@my-monorepo/env/client";
import { useQuery } from "@tanstack/react-query";
import { createTRPCClient, httpBatchStreamLink } from "@trpc/client";
import { Button } from "@my-monorepo/ui/components/button";
import { trpc } from "@/lib/trpc";
import { setLanguage } from "@/lib/i18n";
import { getTheme, setTheme, type AppTheme } from "@/lib/theme";
import { useTranslation, getCurrentLanguage } from "@my-monorepo/i18n";
import type { AppRouter } from "@my-monorepo/api/routers";
import { superjson } from "@my-monorepo/utils";

const authClient = createBetterAuthClient(env.PUBLIC_API_ORIGIN);

const chatClient = createTRPCClient<AppRouter>({
	links: [
		httpBatchStreamLink({
			url: `${env.PUBLIC_API_ORIGIN}/trpc`,
			transformer: superjson,
		}),
	],
});

function App() {
	const { t } = useTranslation();
	const [theme, setThemeState] = useState<AppTheme>(getTheme);
	const { data: helloData, isLoading, isError } = useQuery(trpc.hello.greet.queryOptions());

	const handleSignIn = async () => {
		setAuthStatus("loading");
		setAuthError(null);
		const { error } = await authClient.signIn.email({ email: authEmail, password: authPassword });
		if (error) {
			setAuthStatus("error");
			setAuthError(error.message ?? "Sign in failed");
		} else {
			setAuthStatus("idle");
			setAuthEmail("");
			setAuthPassword("");
		}
	};

	const handleSignUp = async () => {
		setAuthStatus("loading");
		setAuthError(null);
		const { error } = await authClient.signUp.email({
			email: authEmail,
			password: authPassword,
			name: authEmail,
		});
		if (error) {
			setAuthStatus("error");
			setAuthError(error.message ?? "Sign up failed");
		} else {
			setAuthStatus("idle");
			setAuthEmail("");
			setAuthPassword("");
		}
	};

	const handleSignOut = async () => {
		await authClient.signOut();
	};

	const handleSignInSocial = async (provider: "github" | "google") => {
		setAuthStatus("loading");
		setAuthError(null);
		// TODO: wait official tauri deep-link support
		const { data, error } = await authClient.signIn.social({
			provider,
			callbackURL: env.PUBLIC_TAURI_ORIGIN,
		});
		if (error) {
			setAuthStatus("error");
			setAuthError(error.message ?? "Sign in failed");
			return;
		}
		if (!data?.url) {
			setAuthStatus("error");
			setAuthError("Social sign in URL missing");
			return;
		}
		await openUrl(data.url);
		setAuthStatus("idle");
	};

	const toggleLanguage = () => {
		const current = getCurrentLanguage();
		void setLanguage(current === "en" ? "zh" : "en");
	};

	const toggleTheme = () => {
		const themes = ["light", "dark", "auto"] as const;
		const next = themes[(themes.indexOf(theme) + 1) % themes.length]!;
		setTheme(next);
		setThemeState(next);
	};

	const [prompt, setPrompt] = useState("");
	const [assistantText, setAssistantText] = useState("");
	const [streamStatus, setStreamStatus] = useState<"idle" | "streaming" | "done" | "error">("idle");
	const [errorMessage, setErrorMessage] = useState<string | null>(null);

	const { data: session, isPending: sessionPending } = authClient.useSession();
	const [authEmail, setAuthEmail] = useState("");
	const [authPassword, setAuthPassword] = useState("");
	const [authStatus, setAuthStatus] = useState<"idle" | "loading" | "error">("idle");
	const [authError, setAuthError] = useState<string | null>(null);

	const handleSendMessage = async () => {
		const content = prompt.trim();
		if (!content || streamStatus === "streaming") return;

		setAssistantText("");
		setErrorMessage(null);
		setStreamStatus("streaming");

		try {
			let runningText = "";
			const stream = await chatClient.chat.sendMessage.mutate({
				messages: [{ role: "user", content }],
			});

			for await (const chunk of stream) {
				if (chunk.text) {
					runningText += chunk.text;
					setAssistantText(runningText);
				}

				if (chunk.status === "done") {
					setStreamStatus("done");
				}

				if (chunk.status === "error") {
					setStreamStatus("error");
				}
			}
		} catch (error) {
			setStreamStatus("error");
			setErrorMessage(error instanceof Error ? error.message : "Unknown error");
		}
	};

	return (
		<main className="flex min-h-screen flex-col items-center p-4">
			<div className="flex w-full max-w-2xl flex-col gap-4">
				<div className="p-4 border rounded bg-card text-card-foreground">
					<h2 className="text-lg font-bold mb-2">i18n Test</h2>
					<p>{t("common.welcome")}</p>
					<Button type="button" onClick={toggleLanguage} className="mt-2">
						{t("settings.language")}: {getCurrentLanguage().toUpperCase()}
					</Button>
				</div>

				<div className="p-4 border rounded bg-card text-card-foreground">
					<h2 className="text-lg font-bold mb-2">Theme Test</h2>
					<Button type="button" onClick={toggleTheme}>
						{theme}
					</Button>
				</div>

				<div className="p-4 border rounded bg-card text-card-foreground">
					<h2 className="text-lg font-bold mb-2">tRPC Test</h2>
					{isLoading ? (
						<p>{t("common.loading")}</p>
					) : isError ? (
						<p className="text-red-500">{t("common.error")} - API not running?</p>
					) : (
						<p>{helloData?.message}</p>
					)}
				</div>

				<div className="p-4 border rounded bg-card text-card-foreground">
					<h2 className="text-lg font-bold mb-2">Chat Test</h2>
					<div className="flex flex-col gap-3">
						<textarea
							className="min-h-24 rounded border bg-background p-2 text-sm"
							placeholder="Type a message..."
							value={prompt}
							onChange={(event) => setPrompt(event.target.value)}
						/>
						<div className="flex flex-wrap items-center gap-2">
							<Button type="button" onClick={handleSendMessage} disabled={!prompt.trim()}>
								Send
							</Button>
							<Button
								type="button"
								variant="outline"
								onClick={() => {
									setPrompt("");
									setAssistantText("");
									setErrorMessage(null);
									setStreamStatus("idle");
								}}
							>
								Clear
							</Button>
							<span className="text-sm text-muted-foreground">Status: {streamStatus}</span>
						</div>
						{errorMessage ? <p className="text-sm text-red-500">{errorMessage}</p> : null}
						<div className="min-h-30 whitespace-pre-wrap rounded border bg-muted p-3 text-sm">
							{assistantText || "Waiting for response..."}
						</div>
					</div>
				</div>

				<div className="p-4 border rounded bg-card text-card-foreground">
					<h2 className="text-lg font-bold mb-2">Auth Test</h2>
					<div className="flex flex-col gap-3">
						{sessionPending ? (
							<p className="text-sm text-muted-foreground">Loading session...</p>
						) : session?.user ? (
							<div className="flex flex-col gap-2">
								<p className="text-sm">
									Signed in as: <span className="font-medium">{session.user.email}</span>
								</p>
								<Button type="button" variant="outline" onClick={handleSignOut}>
									Sign Out
								</Button>
							</div>
						) : (
							<div className="flex flex-col gap-2">
								<input
									className="rounded border bg-background px-3 py-2 text-sm"
									type="email"
									placeholder="Email"
									value={authEmail}
									onChange={(e) => setAuthEmail(e.target.value)}
								/>
								<input
									className="rounded border bg-background px-3 py-2 text-sm"
									type="password"
									placeholder="Password"
									value={authPassword}
									onChange={(e) => setAuthPassword(e.target.value)}
								/>
								<div className="flex flex-wrap items-center gap-2">
									<Button
										type="button"
										onClick={handleSignIn}
										disabled={!authEmail || !authPassword || authStatus === "loading"}
									>
										Sign In
									</Button>
									<Button
										type="button"
										variant="outline"
										onClick={handleSignUp}
										disabled={!authEmail || !authPassword || authStatus === "loading"}
									>
										Sign Up
									</Button>
									<span className="text-sm text-muted-foreground">Status: {authStatus}</span>
								</div>
								<div className="flex flex-wrap gap-2">
									<Button
										type="button"
										variant="outline"
										onClick={() => handleSignInSocial("github")}
										disabled={authStatus === "loading"}
									>
										Sign In with GitHub
									</Button>
									<Button
										type="button"
										variant="outline"
										onClick={() => handleSignInSocial("google")}
										disabled={authStatus === "loading"}
									>
										Sign In with Google
									</Button>
								</div>
								{authError ? <p className="text-sm text-red-500">{authError}</p> : null}
							</div>
						)}
					</div>
				</div>
			</div>
		</main>
	);
}

export default App;
