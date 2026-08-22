import { describe, it, expect } from "vite-plus/test";
import { getCurrentLanguage, changeLanguage, supportedLanguages } from "./index";

describe("i18n", () => {
	it("should have supported languages", () => {
		expect(supportedLanguages).toContain("en");
		expect(supportedLanguages).toContain("zh");
	});

	it("should get current language", () => {
		const lang = getCurrentLanguage();
		expect(supportedLanguages).toContain(lang);
	});

	it("should change language", async () => {
		await changeLanguage("zh");
		expect(getCurrentLanguage()).toBe("zh");

		await changeLanguage("en");
		expect(getCurrentLanguage()).toBe("en");
	});
});
