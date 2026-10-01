export interface IslandGeometry {
	topInset: number;
	notchWidth: number;
}

export interface UpdateInfo {
	currentVersion: string;
	version: string;
	notes: string | null;
}

export type UpdateProgress =
	| { status: "downloading"; downloaded: number; total: number | null }
	| { status: "downloaded" };
