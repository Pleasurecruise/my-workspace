import { env } from "@my-monorepo/env";
import { createLoggerWithContext } from "@my-monorepo/logger";
import nodemailer from "nodemailer";

const logger = createLoggerWithContext("email");

export interface SendEmailOptions {
	to: string;
	subject: string;
	text?: string;
	html?: string;
}

const transporter = nodemailer.createTransport({
	host: env.MAIL_HOST,
	port: env.MAIL_PORT,
	secure: env.MAIL_SECURE,
	auth: {
		user: env.MAIL_AUTH_USER,
		pass: env.MAIL_AUTH_PASS,
	},
});

export async function sendEmail({ to, subject, text, html }: SendEmailOptions) {
	try {
		const info = await transporter.sendMail({
			from: env.MAIL_FROM,
			to,
			subject,
			text,
			html,
		});
		return { success: true, messageId: info.messageId };
	} catch (error) {
		logger.error("Error sending email:", { error });
		return { success: false, error };
	}
}
