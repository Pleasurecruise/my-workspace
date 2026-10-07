export interface NewspaperDaily {
	date: string;
	generatedAt: string;
	url: string;
	lead: { title: string; paragraph: string } | null;
	sections: Array<{
		label: string;
		items: Array<{ title: string; summary: string; source: string; url: string }>;
	}>;
	flashes: Array<{ title: string; source: string; url: string; publishedAt: string }>;
}
