export interface Logger {
	info: (message: string, data?: object) => void;
	error: (message: string, data?: object) => void;
	warn: (message: string, data?: object) => void;
	debug: (message: string, data?: object) => void;
	trace: (message: string, data?: object) => void;
	fatal: (message: string, data?: object) => void;
}
