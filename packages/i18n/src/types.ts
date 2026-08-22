import type en from "./locales/en";

export type DefaultLanguage = "en";
export type SupportedLanguage = "en" | "zh";
export type TranslationResource = typeof en;

type NestedKeyOf<T, Prefix extends string = ""> = T extends object
	? {
			[K in keyof T & string]: T[K] extends object
				? NestedKeyOf<T[K], `${Prefix}${K}.`>
				: `${Prefix}${K}`;
		}[keyof T & string]
	: never;

export type TranslationKey = NestedKeyOf<TranslationResource>;
