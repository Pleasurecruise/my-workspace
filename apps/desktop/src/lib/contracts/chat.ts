export type ChatPart =
	| { kind: "text"; text: string; html: string }
	| { kind: "thinking"; text: string }
	| {
			kind: "tool";
			id: string;
			name: string;
			arguments: string;
			output: string;
			state: "pending" | "running" | "failed" | "complete";
	  };

export interface ChatSnapshot {
	revision: number;
	connected: boolean;
	busy: boolean;
	model: string;
	messages: Array<{ id: string; role: "user" | "assistant"; parts: ChatPart[] }>;
	error: string | null;
}
