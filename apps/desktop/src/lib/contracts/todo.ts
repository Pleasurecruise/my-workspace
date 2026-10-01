export interface TodoDetails {
	calendar: string;
	startDate: string;
	startTime: string | null;
	endDate: string | null;
	endTime: string | null;
	location: string | null;
}

export interface TodoItem {
	sourceOwned: boolean;
	rollover: boolean;
	id: string;
	description: string | null;
	text: string;
	completed: boolean;
	details: TodoDetails | null;
}

export interface TodoList {
	syncError: string | null;
	date: string;
	items: TodoItem[];
}

export interface CheckIn {
	id: string;
	editable: boolean;
	date: string;
	completed: boolean;
	streak: number;
	total: number;
	days: Array<{ date: string; completed: boolean }>;
}
