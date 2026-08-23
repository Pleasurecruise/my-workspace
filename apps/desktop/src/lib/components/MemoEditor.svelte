<script lang="ts">
	import { Textarea } from "@my-workspace/ui";

	let {
		value = $bindable(""),
		placeholder = "",
		onsubmit,
	}: {
		value: string;
		placeholder: string;
		onsubmit: () => void;
	} = $props();

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			onsubmit();
		}
	}
</script>

<Textarea
	class="min-h-20 resize-y border-0 bg-transparent px-3 py-2 text-sm leading-relaxed shadow-none [field-sizing:content] placeholder:opacity-85 focus-visible:ring-0 focus-visible:ring-offset-0"
	bind:value
	{placeholder}
	onkeydown={handleKeydown}
/>
