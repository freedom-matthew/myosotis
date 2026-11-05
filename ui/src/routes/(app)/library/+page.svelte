<script lang="ts">
	import { deleteCard, editCard, libraryCards, setSuspended, type LibraryCard } from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Progress } from '#lib/components/ui/progress/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import { isTyping } from '#lib/keys.js';
	import { cn } from '#lib/utils.js';

	type Filter = 'all' | 'due' | 'new' | 'low' | 'suspended';
	type FilterOption = { value: Filter; label: string };

	const LOW_RECALL_PERCENT: number = 70;
	const DAY_MS: number = 86_400_000;
	const FILTERS: FilterOption[] = [
		{ value: 'all', label: 'All' },
		{ value: 'due', label: 'Due today' },
		{ value: 'new', label: 'New' },
		{ value: 'low', label: 'Low recall' },
		{ value: 'suspended', label: 'Suspended' }
	];

	let cards: LibraryCard[] = $state([]);
	let search: string = $state('');
	let filter: Filter = $state('all');
	let selectedId: number | null = $state(null);
	let question: string = $state('');
	let answer: string = $state('');
	let error: string | null = $state(null);
	let confirmingDelete: boolean = $state(false);
	let searchInput: HTMLInputElement | null = $state(null);
	let questionInput: HTMLTextAreaElement | null = $state(null);

	const visible: LibraryCard[] = $derived(cards.filter((card) => matches(card)));
	const selected: LibraryCard | undefined = $derived(
		cards.find((card) => card.id === selectedId)
	);
	const edited: boolean = $derived(
		selected !== undefined && (question !== selected.question || answer !== selected.answer)
	);

	function isDueToday(card: LibraryCard): boolean {
		const endOfToday: Date = new Date();
		endOfToday.setHours(23, 59, 59, 999);
		return !card.suspended && new Date(card.next_due) <= endOfToday;
	}

	function passesFilter(card: LibraryCard): boolean {
		if (filter === 'suspended') return card.suspended;
		if (filter === 'due') return isDueToday(card);
		if (filter === 'new') return card.is_new;
		if (filter === 'low') {
			return card.recall_percent !== null && card.recall_percent < LOW_RECALL_PERCENT;
		}
		return true;
	}

	function matches(card: LibraryCard): boolean {
		const wanted: string = search.trim().toLowerCase();
		const haystack: string = [card.question, card.answer, card.source_title, card.excerpt]
			.join(' ')
			.toLowerCase();
		return passesFilter(card) && haystack.includes(wanted);
	}

	function nextLabel(card: LibraryCard): string {
		if (card.suspended) return 'Suspended';
		const days: number = Math.ceil((new Date(card.next_due).getTime() - Date.now()) / DAY_MS);
		if (days <= 0 || isDueToday(card)) return 'today';
		return days === 1 ? '1 day' : `${days} days`;
	}

	function select(card: LibraryCard): void {
		selectedId = card.id;
		question = card.question;
		answer = card.answer;
	}

	async function load(): Promise<void> {
		try {
			cards = await libraryCards();
		} catch (reason) {
			error = String(reason);
		}
	}

	async function act(action: () => Promise<void>): Promise<void> {
		error = null;
		try {
			await action();
		} catch (reason) {
			error = String(reason);
		}
		await load();
	}

	function save(): void {
		if (!selected || question.trim() === '' || answer.trim() === '') return;
		const id: number = selected.id;
		act(() => editCard(id, question, answer));
	}

	function toggleSuspended(): void {
		if (!selected) return;
		const card: LibraryCard = selected;
		act(() => setSuspended(card.id, !card.suspended));
	}

	function confirmDelete(): void {
		if (!selected) return;
		const id: number = selected.id;
		confirmingDelete = false;
		selectedId = null;
		act(() => deleteCard(id));
	}

	function moveSelection(step: number): void {
		const position: number = visible.findIndex((card) => card.id === selectedId);
		const next: LibraryCard | undefined = visible[position + step];
		if (next) select(next);
	}

	$effect(() => {
		load();
	});

	function onkeydown(event: KeyboardEvent): void {
		if (isTyping(event) || confirmingDelete) return;
		if (event.key === '/') {
			event.preventDefault();
			searchInput?.focus();
		} else if (event.key === 'ArrowDown') {
			event.preventDefault();
			moveSelection(1);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			moveSelection(-1);
		} else if (event.key === 'Enter' && selected) {
			event.preventDefault();
			questionInput?.focus();
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="flex h-full flex-col gap-6 px-10 pt-9 pb-6">
	<header class="flex flex-col gap-1">
		<h1 class="font-serif text-3xl font-normal">Library</h1>
		<span class="font-mono text-xs text-muted-foreground">
			{cards.length}
			{cards.length === 1 ? 'card' : 'cards'}
		</span>
	</header>

	{#if error}
		<Notice>{error}</Notice>
	{/if}

	<div class="flex items-center gap-3">
		<Input
			bind:ref={searchInput}
			bind:value={search}
			placeholder="Search questions, answers and sources"
			aria-label="Search"
			class="h-9 max-w-80 bg-card"
		/>
		<ToggleGroup.Root
			type="single"
			value={filter}
			onValueChange={(value: string) => {
				if (value) filter = value as Filter;
			}}
		>
			{#each FILTERS as option (option.value)}
				<ToggleGroup.Item value={option.value} class="text-xs">{option.label}</ToggleGroup.Item>
			{/each}
		</ToggleGroup.Root>
	</div>

	<div class="grid min-h-0 flex-1 grid-cols-[1fr_20rem] gap-6">
		<section class="min-h-0 overflow-y-auto rounded-2xl border bg-card">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head>Question</Table.Head>
						<Table.Head>From</Table.Head>
						<Table.Head class="w-28">Recall now</Table.Head>
						<Table.Head class="w-20">Next</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each visible as card (card.id)}
						<Table.Row
							onclick={() => select(card)}
							class={cn(card.id === selectedId && 'bg-primary-soft hover:bg-primary-soft')}
						>
							<Table.Cell class="max-w-64 truncate font-serif text-base">
								{card.question}
							</Table.Cell>
							<Table.Cell class="max-w-32 truncate text-xs text-muted-foreground">
								{card.source_title ?? '—'}
							</Table.Cell>
							<Table.Cell>
								{#if card.recall_percent !== null}
									<div class="flex items-center gap-2">
										<Progress value={card.recall_percent} max={100} class="h-1 flex-1" />
										<span class="font-mono text-xs text-muted-foreground">
											{card.recall_percent}%
										</span>
									</div>
								{:else}
									<span class="text-xs text-muted-foreground">New</span>
								{/if}
							</Table.Cell>
							<Table.Cell class="font-mono text-xs text-muted-foreground">
								{nextLabel(card)}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
			{#if visible.length === 0}
				<p class="p-6 text-center text-sm text-muted-foreground">No cards here.</p>
			{/if}
		</section>

		{#if selected}
			<aside class="flex min-h-0 flex-col gap-4 overflow-y-auto rounded-2xl border bg-card p-5">
				<div class="flex flex-col gap-2">
					<Label for="question">Question</Label>
					<Textarea
						id="question"
						bind:ref={questionInput}
						bind:value={question}
						class="font-serif text-base"
					/>
				</div>
				<div class="flex flex-col gap-2">
					<Label for="answer">Answer</Label>
					<Textarea id="answer" bind:value={answer} class="font-serif text-base" />
				</div>
				{#if edited}
					<Button onclick={save} class="self-start">Save</Button>
				{/if}
				{#if selected.excerpt}
					<div class="flex flex-col gap-1.5">
						<span class="text-sm font-medium">Context</span>
						<p class="rounded-lg border bg-secondary p-3 font-serif text-base text-card-answer">
							{selected.excerpt}
						</p>
					</div>
				{/if}
				<dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
					<dt class="text-muted-foreground">From</dt>
					<dd>{selected.source_title ?? '—'}</dd>
					<dt class="text-muted-foreground">Reviews</dt>
					<dd class="font-mono">{selected.review_count}</dd>
					<dt class="text-muted-foreground">Next review</dt>
					<dd class="font-mono">{nextLabel(selected)}</dd>
				</dl>
				<div class="flex gap-2">
					<Button variant="secondary" onclick={toggleSuspended} class="border-input">
						{selected.suspended ? 'Resume' : 'Suspend'}
					</Button>
					<Button variant="destructive" onclick={() => (confirmingDelete = true)}>Delete</Button>
				</div>
			</aside>
		{:else}
			<aside
				class="flex items-center justify-center rounded-2xl border border-dashed p-5 text-sm text-muted-foreground"
			>
				Select a card
			</aside>
		{/if}
	</div>
</div>

<AlertDialog.Root bind:open={confirmingDelete}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Delete this card?</AlertDialog.Title>
			<AlertDialog.Description>
				{#if selected && selected.review_count > 0}
					Its {selected.review_count}
					{selected.review_count === 1 ? 'review' : 'reviews'} will be deleted with it. This can't be
					undone.
				{:else}
					This can't be undone.
				{/if}
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Keep it</AlertDialog.Cancel>
			<AlertDialog.Action onclick={confirmDelete}>Delete</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
