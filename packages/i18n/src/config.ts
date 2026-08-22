import i18next, { type i18n as I18n } from "i18next";
import type { SupportedLanguage } from "./types";
import en from "./locales/en";
import zh from "./locales/zh";

const resources = {
	en: { translation: en },
	zh: { translation: zh },
};

const i18n: I18n = i18next.createInstance();

void i18n.init({
	resources,
	lng: "en",
	fallbackLng: "en",
	interpolation: {
		escapeValue: false,
	},
});

export { i18n };

export function changeLanguage(lang: SupportedLanguage) {
	return i18n.changeLanguage(lang);
}

export function getCurrentLanguage(): SupportedLanguage {
	return i18n.language as SupportedLanguage;
}
