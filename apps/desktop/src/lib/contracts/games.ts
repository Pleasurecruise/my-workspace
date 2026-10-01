export type Game = "genshin" | "starRail" | "zzz" | "arknights" | "endfield";

export type GameProvider = "mihoyo" | "skland" | "steam";

export interface GameConnections {
	providers: GameProvider[];
	mihoyo: string[];
	bindings: Array<{ game: Game; accountId: string | null }>;
}

export interface GameLoginQr {
	id: string;
	image: string;
	expiresAt: number;
}

export type GameLoginProgress = "waiting" | "scanned" | "expired" | "complete";

export interface GameNotes {
	account: { game: Game; uid: string; region: string; name: string; roleId: string };
	sampledAt: number;
	meters: Array<{ label: string; current: number; max: number; fullAt: number | null }>;
	tasks: Array<{
		label: string;
		value: string;
		progress: { current: number; total: number } | null;
	}>;
}

export type GameNotesResponse =
	| { status: "ready"; data: GameNotes }
	| { status: "verificationRequired"; code: number; message: string }
	| { status: "refreshRequired"; message: string }
	| { status: "failed"; message: string };

export interface GachaPull {
	id: string;
	pool: string;
	poolName: string;
	itemId: string | null;
	name: string;
	rarity: number;
	time: string;
	isFree: boolean | null;
	isNew: boolean | null;
}

export interface GachaArchive {
	game: Game;
	uid: string | null;
	accounts: Array<{ uid: string; name: string }>;
	total: number;
	added: number;
	syncedAt: number | null;
	firstRecord: string | null;
	lastRecord: string | null;
	pools: Array<{
		id: string;
		name: string;
		total: number;
		highRarity: number;
		rarities: [number, number, number, number, number, number];
		freePulls: number;
		sinceHighRarity: number;
		averageInterval: number | null;
	}>;
	recent: GachaPull[];
	official: StarRailReport | null;
}

export interface StarRailReport {
	pools: Array<{
		id: string;
		name: string;
		total: number;
		sinceHighRarity: number | null;
		fiveStars: Array<{ id: string; itemId: number; name: string; pulls: number; isUp: boolean }>;
	}>;
}

export interface SteamSettings {
	apiKey: string;
	steamId: string;
}

export interface SteamGames {
	name: string;
	profileUrl: string;
	state: number | null;
	playing: string | null;
	ownedGames: number | null;
	recentCount: number | null;
	recentMinutes: number | null;
	totalMinutes: number | null;
	playedGames: number | null;
	mostPlayed: SteamGames["recent"];
	sampledAt: number;
	recent: Array<{
		appId: number;
		name: string;
		playtimeForever: number;
		playtime2weeks: number | null;
	}>;
}
