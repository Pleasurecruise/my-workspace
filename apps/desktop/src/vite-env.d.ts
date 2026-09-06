/// <reference types="vite/client" />

declare module "@my-workspace/ui/styles" {
	const stylesheet: string;
	export default stylesheet;
}

declare module "*.css" {}

declare module "*.html?raw" {
	const html: string;
	export default html;
}
