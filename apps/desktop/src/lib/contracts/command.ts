export type CommandResponse<T> =
	| { status: "ready"; data: T }
	| { status: "failed"; message: string };
