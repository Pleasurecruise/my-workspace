import { describe, expect, it, vi } from "vite-plus/test";
import { sendEmail } from "./email.js";
import nodemailer from "nodemailer";

vi.mock("nodemailer", () => ({
	default: {
		createTransport: vi.fn().mockReturnValue({
			sendMail: vi.fn().mockResolvedValue({ messageId: "test-id" }),
		}),
	},
}));

describe("email", () => {
	it("should send an email successfully", async () => {
		const result = await sendEmail({
			to: "test@example.com",
			subject: "Test Subject",
			text: "Test Text",
		});

		expect(result.success).toBe(true);
		expect(result.messageId).toBe("test-id");
	});

	it("should handle email sending failure", async () => {
		const transporter = nodemailer.createTransport({});
		// oxlint-disable-next-line typescript-eslint/unbound-method
		vi.mocked(transporter.sendMail).mockRejectedValueOnce(new Error("Failed to send"));

		const result = await sendEmail({
			to: "test@example.com",
			subject: "Test Subject",
			text: "Test Text",
		});

		expect(result.success).toBe(false);
		expect(result.error).toBeDefined();
	});
});
