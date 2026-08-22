import { betterAuth } from "better-auth";
import { prismaAdapter } from "@better-auth/prisma-adapter";
import { env } from "@my-monorepo/env";
import { sendEmail } from "@my-monorepo/utils/email";
import prisma from "@my-monorepo/db";

export const auth = betterAuth({
	baseURL: env.PUBLIC_API_ORIGIN,
	secret: env.BETTER_AUTH_SECRET,
	trustedOrigins: [env.PUBLIC_WEB_ORIGIN, env.PUBLIC_TAURI_ORIGIN, env.PUBLIC_API_ORIGIN].filter(
		Boolean,
	) as string[],

	/** if no database is provided, the user data will be stored in memory.
	 * Make sure to provide a database to persist user data **/
	database: prismaAdapter(prisma, {
		provider: "postgresql",
	}),
	emailAndPassword: {
		enabled: true,
		requireEmailVerification: false,
		sendResetPassword: async ({ user, url }) => {
			await sendEmail({
				to: user.email,
				subject: "Reset your password",
				html: `<p>Click the link below to reset your password:</p><p><a href="${url}">${url}</a></p>`,
			});
		},
	},
	// emailVerification: {
	//     sendVerificationEmail: async ( { user, url, token }, request) => {
	//         await sendEmail({
	//             to: user.email,
	//             subject: "Verify your email address",
	//             html: `<a href="${url}">Click the link to verify your email</a>`
	//         });
	//     },
	// },
	socialProviders: {
		...(env.GITHUB_CLIENT_ID && env.GITHUB_CLIENT_SECRET
			? {
					github: {
						clientId: env.GITHUB_CLIENT_ID,
						clientSecret: env.GITHUB_CLIENT_SECRET,
					},
				}
			: {}),
		...(env.GOOGLE_CLIENT_ID && env.GOOGLE_CLIENT_SECRET
			? {
					google: {
						clientId: env.GOOGLE_CLIENT_ID,
						clientSecret: env.GOOGLE_CLIENT_SECRET,
					},
				}
			: {}),
	},
	session: {
		fields: {
			expiresAt: "expiresAt",
			token: "token",
		},
		cookieCache: {
			enabled: true,
			maxAge: 30 * 60, // Cache duration in seconds
		},
	},
});

/**
 * Get the current session from request headers.
 * Use this in server-side handlers (e.g. Hono middleware).
 *
 * For TanStack Start, use createServerFn + getRequestHeaders() instead.
 *
 * @example
 * // In a Hono route handler
 * const session = await getSession(c.req.raw.headers);
 */
export const getSession = async (headers: Headers) => {
	return await auth.api.getSession({ headers });
};

export type Auth = typeof auth;
export type Session = typeof auth.$Infer.Session;
export type User = typeof auth.$Infer.Session.user;
