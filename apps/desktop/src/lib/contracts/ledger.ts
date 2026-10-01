export interface ExpenseEntry {
	description: string | null;
	id: string;
	date: string;
	amountPence: number;
	category: string;
}

export interface ExpenseSnapshot {
	date: string;
	month: string;
	entries: ExpenseEntry[];
	dayTotalPence: number;
	monthTotalPence: number;
	categories: Array<{ category: string; amountPence: number }>;
	days: Array<{ date: string; amountPence: number }>;
	suggestions: string[];
}
