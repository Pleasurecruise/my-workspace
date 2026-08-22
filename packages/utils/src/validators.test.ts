import { describe, expect, it } from "vite-plus/test";

import {
	isAlphanumeric,
	isBase64,
	isChinaMobilePhone,
	isCreditCard,
	isDate,
	isEmail,
	isFloat,
	isHexColor,
	isIP,
	isIPv4,
	isIPv6,
	isIdentityCard,
	isInt,
	isJSON,
	isJWT,
	isMACAddress,
	isMobilePhone,
	isNumeric,
	isPostalCode,
	isStrongPassword,
	isURL,
	isUUID,
} from "./validators.js";

describe("validators", () => {
	it("validates emails", () => {
		expect(isEmail("user@example.com")).toBe(true);
		expect(isEmail("not-an-email")).toBe(false);
	});

	it("validates mobile phones", () => {
		expect(isMobilePhone("2015550123", "en-US")).toBe(true);
		expect(isMobilePhone("123", "en-US")).toBe(false);
	});

	it("validates China mobile phones", () => {
		expect(isChinaMobilePhone("13800138000")).toBe(true);
		expect(isChinaMobilePhone("123")).toBe(false);
	});

	it("validates IP addresses", () => {
		expect(isIP("127.0.0.1")).toBe(true);
		expect(isIP("not-an-ip")).toBe(false);
	});

	it("validates IPv4 addresses", () => {
		expect(isIPv4("127.0.0.1")).toBe(true);
		expect(isIPv4("2001:0db8:85a3:0000:0000:8a2e:0370:7334")).toBe(false);
	});

	it("validates IPv6 addresses", () => {
		expect(isIPv6("2001:0db8:85a3:0000:0000:8a2e:0370:7334")).toBe(true);
		expect(isIPv6("127.0.0.1")).toBe(false);
	});

	it("validates URLs", () => {
		expect(isURL("https://example.com")).toBe(true);
		expect(isURL("not-a-url")).toBe(false);
	});

	it("validates UUIDs", () => {
		expect(isUUID("550e8400-e29b-41d4-a716-446655440000")).toBe(true);
		expect(isUUID("not-a-uuid")).toBe(false);
	});

	it("validates identity cards", () => {
		expect(isIdentityCard("11010519491231002X")).toBe(true);
		expect(isIdentityCard("123")).toBe(false);
	});

	it("validates postal codes", () => {
		expect(isPostalCode("90210", "US")).toBe(true);
		expect(isPostalCode("ABCDE", "US")).toBe(false);
	});

	it("validates credit card numbers", () => {
		expect(isCreditCard("4111111111111111")).toBe(true);
		expect(isCreditCard("1234")).toBe(false);
	});

	it("validates JSON strings", () => {
		expect(isJSON('{"value":1}')).toBe(true);
		expect(isJSON("not-json")).toBe(false);
	});

	it("validates strong passwords", () => {
		expect(isStrongPassword("Aa1!aaaa")).toBe(true);
		expect(isStrongPassword("weak")).toBe(false);
	});

	it("validates dates", () => {
		expect(isDate("2025-02-06")).toBe(true);
		expect(isDate("not-a-date")).toBe(false);
	});

	it("validates numeric values", () => {
		expect(isNumeric("12345")).toBe(true);
		expect(isNumeric("12a")).toBe(false);
	});

	it("validates integers", () => {
		expect(isInt("42")).toBe(true);
		expect(isInt("42.5")).toBe(false);
	});

	it("validates floating point numbers", () => {
		expect(isFloat("42.5")).toBe(true);
		expect(isFloat("nope")).toBe(false);
	});

	it("validates hex colors", () => {
		expect(isHexColor("#ff00ff")).toBe(true);
		expect(isHexColor("zzzz")).toBe(false);
	});

	it("validates MAC addresses", () => {
		expect(isMACAddress("00:1A:2B:3C:4D:5E")).toBe(true);
		expect(isMACAddress("00:1A:2B:3C:4D")).toBe(false);
	});

	it("validates alphanumeric values", () => {
		expect(isAlphanumeric("abc123")).toBe(true);
		expect(isAlphanumeric("abc-123")).toBe(false);
	});

	it("validates base64 strings", () => {
		expect(isBase64("dGVzdA==")).toBe(true);
		expect(isBase64("not-base64")).toBe(false);
	});

	it("validates JWT tokens", () => {
		expect(
			isJWT(
				"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
			),
		).toBe(true);
		expect(isJWT("not-a-jwt")).toBe(false);
	});
});
