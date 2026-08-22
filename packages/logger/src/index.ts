import pino from "pino";
import { env } from "@my-monorepo/env";

export type { Logger } from "./types.js";

/**
 * Check if we're in pretty mode
 */
const isPretty = env.LOG_PRETTY;

/**
 * Create the base pino logger instance
 */
const baseLogger = pino({
	level: env.LOG_LEVEL ?? "info",
	serializers: {
		req: pino.stdSerializers.req,
		res: pino.stdSerializers.res,
		err: pino.stdSerializers.err,
	},
	// Use pretty printing in development, structured JSON in production
	...(isPretty && {
		transport: {
			target: "pino-pretty",
			options: {
				colorize: true,
				translateTime: "HH:MM:ss",
				ignore: "pid,hostname",
				messageFormat: "{msg}",
				hideObject: false,
				singleLine: false,
				useLevelLabels: true,
				levelFirst: true,
				// Ensure output goes to stdout for IDE terminal
				destination: 1,
			},
		},
	}),
});

/**
 * Create a logger adapter that wraps pino to match the existing API
 */
function createLoggerAdapter(pinoLogger: pino.Logger, prefixContext?: string) {
	// Format context with brackets if not already formatted
	const formatContext = (ctx?: string): string => {
		if (!ctx) return "";
		// If already has brackets, use as-is, otherwise wrap in brackets
		if (ctx.startsWith("[") && ctx.endsWith("]")) {
			return ctx;
		}
		return `[${ctx}]`;
	};

	const formattedContext = formatContext(prefixContext);

	const createLogMethod =
		(method: "info" | "error" | "warn" | "debug" | "trace" | "fatal") =>
		(message: string, data?: object) => {
			try {
				const fullMessage = formattedContext ? `${formattedContext} ${message}` : message;
				if (data) {
					pinoLogger[method](data, fullMessage);
				} else {
					pinoLogger[method](fullMessage);
				}
			} catch {
				// Silently ignore logger stream errors to prevent crashes
				// This can happen when pino-pretty transport's stream is closing
			}
		};

	return {
		info: createLogMethod("info"),
		error: createLogMethod("error"),
		warn: createLogMethod("warn"),
		debug: createLogMethod("debug"),
		trace: createLogMethod("trace"),
		fatal: createLogMethod("fatal"),
	};
}

/**
 * Default logger instance
 */
export const logger = createLoggerAdapter(baseLogger);

/**
 * Create a child logger with additional context
 * @param context - Context string to prepend to all log messages
 * @returns A new logger instance with the context
 *
 * @example
 * ```ts
 * const logger = createLoggerWithContext("my-component");
 * logger.info("Processing", { userId: 123 }); // Will log with "my-component" as context
 * ```
 */
export function createLoggerWithContext(context: string) {
	const childLogger = baseLogger.child({ context });
	return createLoggerAdapter(childLogger, context);
}

export default logger;
