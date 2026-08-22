import crypto from "node:crypto";
import { beforeAll, beforeEach, describe, expect, it } from "vite-plus/test";

import {
	decrypt,
	decryptOAuthState,
	encrypt,
	encryptOAuthState,
	fromUrlSafeBase64,
	generateFileKey,
	toUrlSafeBase64,
	verifyFileKey,
} from "./crypto.js";

describe("crypto utils", () => {
	const plaintext = "This is a secret message.";
	const emptyPlaintext = "";
	let validKey: string;

	beforeAll(() => {
		validKey = crypto.randomBytes(32).toString("hex");
	});

	beforeEach(() => {
		process.env.ENCRYPTION_KEY = validKey;
		process.env.FILE_KEY_SECRET = "test-file-key-secret";
	});

	describe("base64 helpers", () => {
		it("round-trips url-safe base64", () => {
			const base64 = Buffer.from("hello world").toString("base64");
			const urlSafe = toUrlSafeBase64(base64);
			const restored = fromUrlSafeBase64(urlSafe);

			expect(restored).toBe(base64);
		});
	});

	describe("encrypt/decrypt", () => {
		it("encrypts and decrypts a string successfully", () => {
			const encrypted = encrypt(plaintext);
			expect(typeof encrypted).toBe("string");
			expect(Buffer.from(encrypted, "base64").toString("base64")).toBe(encrypted);

			const decrypted = decrypt(encrypted);
			expect(decrypted).toBe(plaintext);
		});

		it("encrypts and decrypts an empty string successfully", () => {
			const encrypted = encrypt(emptyPlaintext);
			expect(typeof encrypted).toBe("string");
			expect(Buffer.from(encrypted, "base64").toString("base64")).toBe(encrypted);

			const decrypted = decrypt(encrypted);
			expect(decrypted).toBe(emptyPlaintext);
		});
	});

	describe("oauth state", () => {
		it("encrypts and decrypts OAuth state payloads", () => {
			const state = { teamId: "123", userId: "456" };
			const encrypted = encryptOAuthState(state);
			const decrypted = decryptOAuthState(encrypted);

			expect(decrypted).toEqual(state);
		});

		it("returns null when validation fails", () => {
			const encrypted = encryptOAuthState({ teamId: "123" });
			const decrypted = decryptOAuthState(
				encrypted,
				(parsed): parsed is { teamId: string; userId: string } =>
					typeof (parsed as { userId?: string }).userId === "string",
			);

			expect(decrypted).toBeNull();
		});
	});

	describe("file key jwt", () => {
		it("generates and verifies a file key", async () => {
			const token = await generateFileKey("team-123");
			const teamId = await verifyFileKey(token);

			expect(teamId).toBe("team-123");
		});

		it("returns null for invalid tokens", async () => {
			const teamId = await verifyFileKey("not-a-token");

			expect(teamId).toBeNull();
		});
	});
});
