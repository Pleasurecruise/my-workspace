export type MusicProvider = "spotify" | "qqMusic";

export interface QqQr {
	image: string;
}

export type QqLoginStatus =
	| { status: "waiting" }
	| { status: "scanned" }
	| { status: "complete" }
	| { status: "expired" };

export interface MusicTrack {
	id: string;
	name: string;
	artists: string[];
	album: string;
	durationMs: number;
	addedAt: string;
	coverKey: string | null;
}

export interface MusicPlayback {
	trackId: string | null;
	playing: boolean;
	progressMs: number;
	durationMs: number;
	order: "sequential" | "repeatOne" | "shuffle";
}

export interface MusicLyrics {
	lines: Array<{ startMs: number | null; text: string }>;
	synced: boolean;
	instrumental: boolean;
}
