import { changeLanguage } from "@my-monorepo/i18n";
import type { SupportedLanguage } from "@my-monorepo/i18n";

const STORAGE_KEY = "language";

export function initI18n() {
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === "en" || stored === "zh") {
		void changeLanguage(stored);
		return;
	}

	const browserLang = navigator.language.toLowerCase();
	if (browserLang.startsWith("zh")) {
		void changeLanguage("zh");
	}
}

export function setLanguage(lang: SupportedLanguage) {
	localStorage.setItem(STORAGE_KEY, lang);
	return changeLanguage(lang);
}
