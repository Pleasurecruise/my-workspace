import { useSyncExternalStore } from "react";
import { i18n } from "./config";
import type { TranslationKey } from "./types";

function subscribe(callback: () => void) {
	i18n.on("languageChanged", callback);
	return () => i18n.off("languageChanged", callback);
}

function getSnapshot() {
	return i18n.language;
}

export function useTranslation() {
	const language = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

	const t = (key: TranslationKey): string => i18n.t(key);

	return { t, language };
}
