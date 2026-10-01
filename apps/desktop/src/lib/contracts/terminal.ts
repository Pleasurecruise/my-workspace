export interface SshDevice {
	id: string;
	name: string;
	dnsName: string;
	address: string;
	os: string;
	online: boolean | null;
	username: string;
}

export type TerminalTarget = { kind: "local" } | { kind: "ssh"; device: SshDevice };

export type TerminalConnection =
	| { kind: "local" }
	| { kind: "ssh"; deviceId: string; username: string };

export interface SshSnapshot {
	devices: SshDevice[];
	error: string | null;
}

export type TerminalOutput =
	| { kind: "data"; bytes: number[] }
	| { kind: "exit"; code: number | null }
	| { kind: "error"; message: string };
