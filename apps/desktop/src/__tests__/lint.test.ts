// @vitest-environment node
import { resolve } from "node:path";
import { ESLint } from "eslint";
import { expect, it } from "vite-plus/test";

const linter = new ESLint({ cwd: resolve(import.meta.dirname, "../../../..") });

it.each([
	["apps/desktop/src/LintProbe.js", "document", "process"],
	["packages/ui/src/LintProbe.js", "document", "process"],
	["apps/desktop/svelte.config.js", "process", "document"],
	["eslint.config.js", "process", "document"],
])("isolates runtime globals in %s", async (filePath, allowed, forbidden) => {
	const results = await linter.lintText(`console.log(${allowed}, ${forbidden});`, { filePath });
	expect(results.flatMap((result) => result.messages)).toEqual([
		expect.objectContaining({ ruleId: "no-undef", message: `'${forbidden}' is not defined.` }),
	]);
});

it.each(["apps/desktop/src", "packages/ui/src"])(
	"checks Svelte template classes in %s",
	async (directory) => {
		const results = await linter.lintText('<div class="p-[13px]">Probe</div>', {
			filePath: `${directory}/LintProbe.svelte`,
		});
		expect(results.flatMap((result) => result.messages.map((message) => message.ruleId))).toContain(
			"shadcn/no-arbitrary-values",
		);
	},
);

it("accepts Svelte runes, TypeScript props, and scale classes", async () => {
	const results = await linter.lintText(
		'<script lang="ts">let { count }: { count: number } = $props(); const doubled = $derived(count * 2);</script><div class="p-4">{doubled}</div>',
		{ filePath: "apps/desktop/src/LintProbe.svelte" },
	);
	expect(results.flatMap((result) => result.messages)).toEqual([]);
});

it("checks Svelte list keys", async () => {
	const results = await linter.lintText("{#each [1, 2] as item}<span>{item}</span>{/each}", {
		filePath: "apps/desktop/src/LintProbe.svelte",
	});
	expect(results.flatMap((result) => result.messages.map((message) => message.ruleId))).toContain(
		"svelte/require-each-key",
	);
});

it("does not allow inline comments to suppress template checks", async () => {
	const results = await linter.lintText(
		'<!-- eslint-disable-next-line shadcn/no-arbitrary-values -->\n<div class="p-[13px]">Probe</div>',
		{ filePath: "apps/desktop/src/LintProbe.svelte" },
	);
	expect(results.flatMap((result) => result.messages.map((message) => message.ruleId))).toContain(
		"shadcn/no-arbitrary-values",
	);
});
