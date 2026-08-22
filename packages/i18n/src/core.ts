import { i18n, changeLanguage, getCurrentLanguage } from "./config";
import type { TranslationKey } from "./types";

export { i18n, changeLanguage, getCurrentLanguage };

export function t(key: TranslationKey): string {
	return i18n.t(key);
}

export type { SupportedLanguage, TranslationKey, TranslationResource } from "./types";

export const supportedLanguages = ["en", "zh"] as const;
