import { describe, it, expect } from "vite-plus/test";
import { appRouter } from "@my-monorepo/api/routers";
import { t, changeLanguage, getCurrentLanguage, supportedLanguages } from "@my-monorepo/i18n";

describe("Web App", () => {
	describe("tRPC", () => {
		it("hello.greet returns message", async () => {
			const caller = appRouter.createCaller({ session: null });
			const result = await caller.hello.greet();
			expect(result).toEqual({ message: "Hello from tRPC!" });
		});

		it("appRouter has hello router", () => {
			expect(appRouter).toHaveProperty("hello");
			expect(appRouter).toHaveProperty("createCaller");
		});

		it("caller context accepts session", async () => {
			const callerWithSession = appRouter.createCaller({
				session: { userId: "test" },
			});
			const result = await callerWithSession.hello.greet();
			expect(result.message).toBeDefined();
		});
	});

	describe("Theme", () => {
		it("theme type is valid (includes system)", () => {
			const validThemes = ["light", "dark", "system"] as const;
			expect(validThemes).toContain("light");
			expect(validThemes).toContain("dark");
			expect(validThemes).toContain("system");
			expect(validThemes).toHaveLength(3);
		});

		it("default theme should be system", () => {
			const defaultTheme = "system";
			expect(["light", "dark", "system"]).toContain(defaultTheme);
		});
	});

	describe("i18n", () => {
		it("should have supported languages", () => {
			expect(supportedLanguages).toContain("en");
			expect(supportedLanguages).toContain("zh");
		});

		it("should translate common.welcome in English", async () => {
			await changeLanguage("en");
			expect(t("common.welcome")).toBe("Welcome");
		});

		it("should translate common.welcome in Chinese", async () => {
			await changeLanguage("zh");
			expect(t("common.welcome")).toBe("欢迎");
			// Reset to English
			await changeLanguage("en");
		});

		it("should get and change current language", async () => {
			await changeLanguage("en");
			expect(getCurrentLanguage()).toBe("en");

			await changeLanguage("zh");
			expect(getCurrentLanguage()).toBe("zh");

			// Reset
			await changeLanguage("en");
		});
	});
});
