<script lang="ts">
	import type { CardKind } from '#lib/api.js';
	import { splitGap } from '#lib/cloze.js';
	import { cn } from '#lib/utils.js';

	let {
		kind,
		question,
		revealed = true
	}: { kind: CardKind; question: string; revealed?: boolean } = $props();

	const parts: ReturnType<typeof splitGap> = $derived(kind === 'cloze' ? splitGap(question) : null);
</script>

{#if parts}
	{parts.before}<span
		class={cn('rounded-sm px-1.5', revealed ? 'bg-primary-tint' : 'bg-primary-soft text-primary')}
		>{revealed ? parts.gap : '      '}</span
	>{parts.after}
{:else}
	{question}
{/if}
