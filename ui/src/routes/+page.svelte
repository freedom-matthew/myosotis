<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	type Card = { question: string; answer: string };

	const cards = invoke<Card[]>('list_cards');
</script>

<h1>Myosotis</h1>

{#await cards}
	<p>Loading…</p>
{:then cards}
	{#if cards.length === 0}
		<p>No cards yet.</p>
	{:else}
		<ul>
			{#each cards as card}
				<li><strong>{card.question}</strong> — {card.answer}</li>
			{/each}
		</ul>
	{/if}
{:catch error}
	<p>Could not load cards: {error}</p>
{/await}
