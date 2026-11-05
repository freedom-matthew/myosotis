<script lang="ts">
	import { goto } from '$app/navigation';
	import { libraryCards, type LibraryCard } from '#lib/api.js';
	import { withoutGapMarks } from '#lib/cloze.js';
	import * as Command from '#lib/components/ui/command/index.js';

	type Action = { label: string; route: string };

	const ACTIONS: Action[] = [
		{ label: 'Start review', route: '/review' },
		{ label: 'New source', route: '/new' },
		{ label: 'Today', route: '/' },
		{ label: 'Inbox', route: '/inbox' },
		{ label: 'Library', route: '/library' },
		{ label: 'Tidy up', route: '/tidy' },
		{ label: 'Settings', route: '/settings' }
	];

	let { open = $bindable(false) }: { open?: boolean } = $props();
	let cards: LibraryCard[] = $state([]);

	$effect(() => {
		if (open) libraryCards().then((result: LibraryCard[]) => (cards = result));
	});

	function go(route: string): void {
		open = false;
		goto(route);
	}
</script>

<Command.Dialog bind:open class="max-w-140 shadow-popover">
	<Command.Input placeholder="Search actions and cards" />
	<Command.List>
		<Command.Empty>Nothing found.</Command.Empty>
		<Command.Group heading="Actions">
			{#each ACTIONS as action (action.route)}
				<Command.Item onSelect={() => go(action.route)}>{action.label}</Command.Item>
			{/each}
		</Command.Group>
		{#if cards.length > 0}
			<Command.Group heading="Cards">
				{#each cards as card (card.id)}
					<Command.Item
						value={`${card.id} ${card.question} ${card.answer} ${card.source_title ?? ''}`}
						onSelect={() => go(`/library?card=${card.id}`)}
					>
						<span class="truncate">{withoutGapMarks(card.question)}</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
