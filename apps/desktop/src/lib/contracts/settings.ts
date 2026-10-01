export interface UgosConfiguration {
	username: string;
	password: string;
}

export interface R2Configuration {
	accessKeyId: string;
	secretAccessKey: string;
}

export interface ApiConfiguration {
	service: "memos" | "moment" | "knowledge";
	apiKey: string;
}

export interface CodexResets {
	enabled: boolean;
}

export interface NotionCalendar {
	viewUrl: string;
}

export interface NtfyConfig {
	token: string;
	development: boolean;
}

export interface TelegramCredentials {
	apiId: number;
	apiHash: string;
	channelUsername: string;
}

export type TelegramAuthorizationStatus =
	| { status: "disconnected" }
	| { status: "ready" }
	| { status: "codeRequired" }
	| { status: "passwordRequired"; hint: string | null };

export type StoredConfiguration<T> = { status: "missing" } | { status: "ready"; data: T };

export interface ConfigurationStatus {
	ugos: StoredConfiguration<UgosConfiguration>;
	r2: StoredConfiguration<R2Configuration>;
	api: {
		memos: StoredConfiguration<string>;
		moment: StoredConfiguration<string>;
		knowledge: StoredConfiguration<string>;
	};
	ntfy: StoredConfiguration<NtfyConfig>;
	ntfyDev: boolean;
	notionCalendar: StoredConfiguration<NotionCalendar>;
	codexResets: CodexResets;
	appLock: StoredConfiguration<string>;
	appLockDev: boolean;
	spotify: StoredConfiguration<string>;
	qqMusic: StoredConfiguration<string>;
	publication: { telegram: boolean; x: boolean };
}
