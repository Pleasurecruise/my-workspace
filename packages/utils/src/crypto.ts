import crypto from "node:crypto";
import { env } from "@my-monorepo/env";
import * as jose from "jose";

/**
 * Converts standard base64 to URL-safe base64 by replacing +/ and trimming padding.
 */
export function toUrlSafeBase64(base64: string): string {
	return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/**
 * Converts URL-safe base64 back to standard base64 by restoring +/ and padding.
 */
export function fromUrlSafeBase64(urlSafeBase64: string): string {
	let base64 = urlSafeBase64.replace(/-/g, "+").replace(/_/g, "/");
	const padding = base64.length % 4;
	if (padding) {
		base64 += "=".repeat(4 - padding);
	}
	return base64;
}

/**
 * Encrypts a JSON-serializable OAuth state for URL usage with AES-256-GCM and URL-safe base64.
 */
export function encryptOAuthState<T>(payload: T): string {
	const encrypted = encrypt(JSON.stringify(payload));
	return toUrlSafeBase64(encrypted);
}

/**
 * Decrypts an OAuth state and optionally validates it, returning null on failure.
 */
export function decryptOAuthState<T>(
	encryptedState: string,
	validate?: (parsed: unknown) => parsed is T,
): T | null {
	try {
		const standardBase64 = fromUrlSafeBase64(encryptedState);
		const decrypted = decrypt(standardBase64);
		const parsed = JSON.parse(decrypted);

		if (validate && !validate(parsed)) {
			return null;
		}

		return parsed as T;
	} catch {
		return null;
	}
}

const ALGORITHM = "aes-256-gcm";
const IV_LENGTH = 16;
const AUTH_TAG_LENGTH = 16;

function getKey(): Buffer {
	const key = env.ENCRYPTION_KEY;
	if (!key) {
		throw new Error("ENCRYPTION_KEY environment variable is not set.");
	}
	if (Buffer.from(key, "hex").length !== 32) {
		throw new Error("ENCRYPTION_KEY must be a 64-character hex string (32 bytes).");
	}
	return Buffer.from(key, "hex");
}

/**
 * Encrypts a plaintext string using AES-256-GCM and returns a base64 payload.
 */
export function encrypt(text: string): string {
	const key = getKey();
	const iv = crypto.randomBytes(IV_LENGTH);
	const cipher = crypto.createCipheriv(ALGORITHM, key, iv);

	let encrypted = cipher.update(text, "utf8", "hex");
	encrypted += cipher.final("hex");

	const authTag = cipher.getAuthTag();

	return Buffer.concat([iv, authTag, Buffer.from(encrypted, "hex")]).toString("base64");
}

/**
 * Decrypts an AES-256-GCM base64 payload back to plaintext.
 */
export function decrypt(encryptedPayload: string): string {
	const key = getKey();

	if (!encryptedPayload) {
		throw new Error("Invalid encrypted payload: must be a non-empty string");
	}

	const dataBuffer = Buffer.from(encryptedPayload, "base64");
	const minLength = IV_LENGTH + AUTH_TAG_LENGTH;

	if (dataBuffer.length < minLength) {
		throw new Error(
			`Invalid encrypted payload: too short. Expected at least ${minLength} bytes, got ${dataBuffer.length}`,
		);
	}

	const iv = dataBuffer.subarray(0, IV_LENGTH);
	const authTag = dataBuffer.subarray(IV_LENGTH, IV_LENGTH + AUTH_TAG_LENGTH);
	const encryptedText = dataBuffer.subarray(IV_LENGTH + AUTH_TAG_LENGTH);

	if (authTag.length !== AUTH_TAG_LENGTH) {
		throw new Error(
			`Invalid auth tag length: expected ${AUTH_TAG_LENGTH} bytes, got ${authTag.length}`,
		);
	}

	const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
	decipher.setAuthTag(authTag);

	let decrypted = decipher.update(encryptedText.toString("hex"), "hex", "utf8");
	decrypted += decipher.final("utf8");

	return decrypted;
}

export function hash(str: string): string {
	return crypto.createHash("sha256").update(str).digest("hex");
}

/**
 * Generates a short-lived JWT containing the teamId for file access.
 */
export async function generateFileKey(teamId: string): Promise<string> {
	const secret = env.FILE_KEY_SECRET;
	if (!secret) {
		throw new Error("FILE_KEY_SECRET environment variable is not set.");
	}
	const secretKey = new TextEncoder().encode(secret);
	return await new jose.SignJWT({ teamId })
		.setProtectedHeader({ alg: "HS256" })
		.setExpirationTime("7d")
		.sign(secretKey);
}

/**
 * Verifies a file key JWT and returns the teamId if valid.
 */
export async function verifyFileKey(token: string): Promise<string | null> {
	try {
		const secret = env.FILE_KEY_SECRET;
		if (!secret) {
			return null;
		}
		const secretKey = new TextEncoder().encode(secret);
		const { payload } = await jose.jwtVerify(token, secretKey);
		return (payload.teamId as string) || null;
	} catch {
		return null;
	}
}
