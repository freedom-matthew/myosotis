<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { addCard, type CardKind, type Reverse } from '#lib/api.js';
	import { addCardDialog } from '#lib/add-card.svelte.js';
	import { KIND_LABELS } from '#lib/card-kind.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import { cn } from '#lib/utils.js';

	type KindCopy = { hint: string; question: string; answer: string };

	const KINDS: CardKind[] = ['question', 'concept', 'describe'];
	const COPY: Record<CardKind, KindCopy> = {
		question: { hint: 'A question with a short answer.', question: 'Question', answer: 'Answer' },
		concept: {
			hint: 'A term you want both ways round. Write each direction as its own question.',
			question: 'Question',
			answer: 'Answer'
		},
		describe: {
			hint: 'An open question you answer in your own words. Claude marks it against the model answer.',
			question: 'Question',
			answer: 'Model answer'
		}
	};

	let kind: CardKind = $state<CardKind>('question');
	let question: string = $state('');
	let answer: string = $state('');
	let reverseQuestion: string = $state('');
	let reverseAnswer: string = $state('');
	let saving: boolean = $state(false);
	let error: string | null = $state(null);
	let questionField: HTMLTextAreaElement | null = $state(null);

	const copy: KindCopy = $derived(COPY[kind]);
	const canAdd: boolean = $derived(
		question.trim() !== '' &&
			answer.trim() !== '' &&
			(kind !== 'concept' || (reverseQuestion.trim() !== '' && reverseAnswer.trim() !== '')) &&
			!saving
	);

	$effect(() => {
		if (!addCardDialog.open) error = null;
	});

	async function add(): Promise<void> {
		if (!canAdd) return;
		saving = true;
		error = null;
		try {
			const reverse: Reverse | null =
				kind === 'concept' ? { question: reverseQuestion, answer: reverseAnswer } : null;
			await addCard({ kind, question, answer, reverse });
			question = '';
			answer = '';
			reverseQuestion = '';
			reverseAnswer = '';
			addCardDialog.added += 1;
			toast(kind === 'concept' ? 'Card added, both ways round' : 'Card added');
			questionField?.focus();
		} catch (reason) {
			error = String(reason);
		}
		saving = false;
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			add();
		}
	}
</script>

<Dialog.Root bind:open={addCardDialog.open}>
	<Dialog.Content
		{onkeydown}
		showCloseButton={false}
		onOpenAutoFocus={(event: Event) => {
			event.preventDefault();
			questionField?.focus();
		}}
		class="gap-6 p-7 shadow-popover sm:max-w-140"
	>
		<Dialog.Header class="gap-1.5">
			<Dialog.Title class="font-serif text-2xl leading-tight font-normal">Add card</Dialog.Title>
			<Dialog.Description>It skips the Inbox and joins your reviews as a new card.</Dialog.Description>
		</Dialog.Header>

		{#if error}
			<Notice>{error}</Notice>
		{/if}

		<div class="flex flex-col gap-2">
			<ToggleGroup.Root
				type="single"
				value={kind}
				onValueChange={(value: string) => {
					if (value) kind = value as CardKind;
				}}
				class="w-full"
			>
				{#each KINDS as option (option)}
					<ToggleGroup.Item value={option} class="min-w-0 flex-1 shrink px-2 text-xs">{KIND_LABELS[option]}</ToggleGroup.Item>
				{/each}
			</ToggleGroup.Root>
			<span class="text-xs text-muted-foreground">{copy.hint}</span>
		</div>

		<div class="flex flex-col gap-2">
			<Label for="new-card-question">{copy.question}</Label>
			<Textarea
				id="new-card-question"
				bind:ref={questionField}
				bind:value={question}
				class="min-h-20 bg-card font-serif text-base"
			/>
		</div>
		<div class="flex flex-col gap-2">
			<Label for="new-card-answer">{copy.answer}</Label>
			<Textarea
				id="new-card-answer"
				bind:value={answer}
				class={cn('bg-card font-serif text-base', kind === 'describe' ? 'min-h-32' : 'min-h-20')}
			/>
		</div>
		{#if kind === 'concept'}
			<div class="flex flex-col gap-2">
				<Label for="new-card-reverse-question">Other way round: question</Label>
				<Textarea
					id="new-card-reverse-question"
					bind:value={reverseQuestion}
					class="min-h-20 bg-card font-serif text-base"
				/>
			</div>
			<div class="flex flex-col gap-2">
				<Label for="new-card-reverse-answer">Other way round: answer</Label>
				<Textarea
					id="new-card-reverse-answer"
					bind:value={reverseAnswer}
					class="min-h-20 bg-card font-serif text-base"
				/>
			</div>
		{/if}

		<div class="flex items-center justify-between gap-4">
			<span class="text-xs text-muted-foreground">Stays open for the next one</span>
			<div class="flex gap-2">
				<Button variant="ghost" onclick={() => (addCardDialog.open = false)}>Done</Button>
				<Button onclick={add} disabled={!canAdd} class="gap-2">
					Add card
					<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground"><span>⌘</span><span>↵</span></Kbd>
				</Button>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
