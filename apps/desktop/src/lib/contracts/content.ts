import type { CommandResponse } from "./command";

export type Channel = "memos" | "moment" | "knowledge";

export interface Memo {
	id: string;
	r2Key: string;
	content: string;
	tags: string[];
	createdAt: string;
	updatedAt: string;
	visibility: "public" | "private";
	pinned: boolean;
	favorite: boolean;
	archived: boolean;
}

export interface MemoView extends Memo {
	html: string;
	metadataComplete: boolean;
}

export interface PublishedPost {
	provider: "telegram" | "x";
	externalId: string;
	url: string | null;
}

export interface MemoTagCount {
	name: string;
	count: number;
}

export type MemoUpdate =
	| { content: string; visibility: "public" | "private" }
	| { tags: string[] }
	| { pinned: boolean }
	| { favorite: boolean }
	| { archived: boolean };

export interface PhotoItem {
	id: string;
	url: string;
	thumbnailUrl: string;
	r2Key: string;
	thumbnailR2Key: string;
	thumbHash: string | null;
	title: string;
	width: number;
	height: number;
	aspectRatio: number | null;
	tags: string[];
	date: string | null;
	description: string | null;
	size: number | null;
	format: string | null;
	geo: { lat: number; lng: number } | null;
}

export interface PhotoMetadata {
	capturedAt: string | null;
	geo: { lat: number; lng: number } | null;
}

export interface PhotoUpload {
	title: string;
	description: string | null;
	tags: string[];
	date: string | null;
	geo: { lat: number; lng: number } | null;
}

export interface PhotoUpdate {
	title: string;
	description: string;
	tags: string[];
}

export type ChannelView =
	| {
			channel: "memos";
			memos: MemoView[];
			nextCursor: string | null;
	  }
	| {
			channel: "moment";
			photos: PhotoItem[];
			total: number;
	  }
	| {
			channel: "knowledge";
			knowledge: KnowledgeEntry[];
			newspaper: NewspaperIssues;
			nextCursor: string | null;
	  };

export interface TocEntry {
	id: string;
	text: string;
	depth: number;
}

export interface KnowledgeEntry {
	id: string;
	title: string;
	summary: string;
	tags: string[];
	visibility: "private" | "public";
	contentHash: string;
	createdAt: string;
	updatedAt: string;
	newspaperEdition: "developer" | "personal" | null;
}

export interface ReadingStats {
	wordCount: number;
	readingMinutes: number;
}

export interface KnowledgeDocument extends KnowledgeEntry {
	source: string;
	html: string;
	toc: TocEntry[];
	stats: ReadingStats;
}

export interface NewspaperIssues {
	developer: string | null;
	personal: string | null;
}

export interface MarkdownSpan {
	start: number;
	end: number;
}

export interface KnowledgeDraft {
	title: string;
	summary: string;
	body: string;
	tags: string[];
}

export interface KnowledgeUpdate extends KnowledgeDraft {
	expectedHash: string;
	expectedUpdatedAt: string;
	visibility?: KnowledgeDocument["visibility"];
}

export interface InitialViews {
	memos: CommandResponse<ChannelView>;
	moment: CommandResponse<ChannelView>;
	knowledge: CommandResponse<ChannelView>;
}
