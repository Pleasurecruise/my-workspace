export interface NtfyNotification {
	id: string;
	topic: string;
	source: string;
	title: string | null;
	message: string;
	timestamp: number;
	tags: string[];
}
