import { afterEach, describe, expect, it, vi } from "vite-plus/test";

import { formatAmount, formatDate, getInitials } from "./format.js";

describe("format utils", () => {
	afterEach(() => {
		vi.useRealTimers();
	});

	describe("formatAmount", () => {
		it("formats currency with defaults", () => {
			const result = formatAmount({ currency: "USD", amount: 1234.5 });

			expect(result).toBe("$1,234.50");
		});

		it("returns undefined when currency is missing", () => {
			const result = formatAmount({ currency: "", amount: 10 });

			expect(result).toBeUndefined();
		});

		it("respects custom fraction digits", () => {
			const result = formatAmount({
				currency: "USD",
				amount: 12.34,
				minimumFractionDigits: 0,
				maximumFractionDigits: 0,
			});

			expect(result).toBe("$12");
		});
	});

	describe("formatDate", () => {
		it("uses short month format for current year", () => {
			vi.useFakeTimers();
			vi.setSystemTime(new Date("2026-02-06T08:00:00Z"));

			const result = formatDate("2026-02-06");

			expect(result).toBe("Feb 6");
		});

		it("uses provided format when year check is disabled", () => {
			const result = formatDate("2026-02-06", "yyyy/MM/dd", false);

			expect(result).toBe("2026/02/06");
		});

		it("falls back to default format for other years", () => {
			vi.useFakeTimers();
			vi.setSystemTime(new Date("2026-02-06T08:00:00Z"));

			const result = formatDate("2025-01-01");

			expect(result).toBe("01/01/2025");
		});
	});

	describe("getInitials", () => {
		it("returns the first two characters for multi-character names", () => {
			expect(getInitials("John Doe")).toBe("JO");
		});

		it("returns a single character for one-letter names", () => {
			expect(getInitials("a")).toBe("A");
		});

		it("uppercases and returns two letters for short names", () => {
			expect(getInitials("li")).toBe("LI");
		});
	});
});
