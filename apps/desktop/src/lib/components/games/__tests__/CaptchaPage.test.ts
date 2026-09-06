import html from "../../../../../src-tauri/src/gaming/captcha.html?raw";
import { runInNewContext } from "node:vm";
import { expect, it, vi } from "vite-plus/test";

function setup() {
	const script = html.match(/<script>([\s\S]*)<\/script>/)?.[1];
	if (script === undefined) throw new Error("Missing captcha script");
	const callbacks = { ready: () => {}, success: () => {}, error: () => {} };
	const status = { textContent: "" };
	const sdk = { src: "", onload: () => {}, onerror: () => {} };
	const navigate = vi.fn<(url: string) => void>();
	const verify = vi.fn();
	const proof = {
		geetest_challenge: "challenge",
		geetest_validate: "validate",
		geetest_seccode: "validate|jordan",
	};
	const widget = {
		onReady(callback: () => void) {
			callbacks.ready = callback;
		},
		onSuccess(callback: () => void) {
			callbacks.success = callback;
		},
		onError(callback: () => void) {
			callbacks.error = callback;
		},
		verify,
		getValidate: () => proof,
	};
	const initGeetest = vi.fn(
		(_options: { gt: string; challenge: string }, ready: (value: typeof widget) => void) =>
			ready(widget),
	);
	const context = {
		document: {
			createElement: () => sdk,
			getElementById: () => status,
			head: { appendChild: vi.fn() },
		},
		location: {
			set href(url: string) {
				navigate(url);
			},
		},
		initGeetest,
	};
	runInNewContext(
		`window = this;\n${script.replace("__VESPER_CAPTCHA__", JSON.stringify({ gt: "gt", challenge: "challenge" }))}`,
		context,
	);
	return { sdk, callbacks, status, navigate, verify, initGeetest, proof };
}

it("submits a human-completed proof once and never submits on load", () => {
	const state = setup();
	state.sdk.onload();
	state.callbacks.ready();
	expect(state.verify).toHaveBeenCalledOnce();
	expect(state.navigate).not.toHaveBeenCalled();
	state.callbacks.success();
	state.callbacks.success();
	expect(state.navigate).toHaveBeenCalledOnce();
	const call = state.navigate.mock.calls[0];
	if (call === undefined) throw new Error("Missing proof callback");
	const url = new URL(call[0]);
	expect(url.protocol).toBe("vesper-captcha:");
	const data = url.searchParams.get("data");
	if (data === null) throw new Error("Missing proof data");
	expect(JSON.parse(data)).toEqual(state.proof);
});

it("reports SDK errors without starting a verification submission", () => {
	const state = setup();
	state.sdk.onerror();
	expect(state.status.textContent).toContain("failed to load");
	expect(state.initGeetest).not.toHaveBeenCalled();
	expect(state.navigate).not.toHaveBeenCalled();
});
