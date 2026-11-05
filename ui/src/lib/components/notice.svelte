<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<div
	class="flex items-center justify-between gap-4 rounded-lg bg-warn-soft px-4 py-3 text-sm text-warn-foreground"
	role="alert"
>
	{@render children()}
</div>
