<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';
	import { FileText, Undo2, X } from '@lucide/svelte';
	import {
		editCard,
		explainCard,
		gradeCard,
		reviewSession,
		setSuspended,
		todaySummary,
		undoGrade,
		type Grade,
		type SessionCard,
		type TodaySummary
	} from '#lib/api.js';
	import CardQuestion from '#lib/components/card-question.svelte';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Progress } from '#lib/components/ui/progress/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import { isTyping } from '#lib/keys.js';
	import { cn } from '#lib/utils.js';

	const GRADE_LABELS: Record<Grade, string> = {
		again: 'Again',
		hard: 'Hard',
		good: 'Good',
		easy: 'Easy'
	};

	let queue: SessionCard[] = $state([]);
	let reviewed: number = $state(0);
	let loaded: boolean = $state(false);
	let revealed: boolean = $state(false);
	let showContext: boolean = $state(false);
	let saving: boolean = $state(false);
	let error: string | null = $state(null);
	let gradedCards: number[] = $state([]);
	let explanation: string | null = $state(null);
	let nextDue: string | null = $state(null);
	let explaining: boolean = $state(false);
	let editing: boolean = $state(false);
	let editQuestion: string = $state('');
	let editAnswer: string = $state('');

	const card: SessionCard | undefined = $derived(queue[0]);
	const total: number = $derived(reviewed + queue.length);

	const skipNew: boolean = page.url.searchParams.has('skip_new');

	async function loadNextDue(): Promise<void> {
		const summary: TodaySummary = await todaySummary();
		nextDue = summary.next_due;
	}

	async function loadQueue(): Promise<void> {
		const cards: SessionCard[] = await reviewSession();
		queue = skipNew ? cards.filter((item) => !item.is_new) : cards;
		if (queue.length === 0) loadNextDue();
		revealed = false;
		showContext = false;
		explanation = null;
		explaining = false;
		loaded = true;
	}

	$effect(() => {
		loadQueue().catch((reason: unknown) => (error = String(reason)));
	});

	async function grade(choice: Grade): Promise<void> {
		if (!card || !revealed || saving) return;
		saving = true;
		error = null;
		try {
			await gradeCard(card.id, choice);
			gradedCards.push(card.id);
			reviewed += 1;
			await loadQueue();
		} catch (reason) {
			error = String(reason);
		}
		saving = false;
	}

	async function undo(): Promise<void> {
		const last: number | undefined = gradedCards.at(-1);
		if (last === undefined || saving) return;
		saving = true;
		error = null;
		try {
			await undoGrade(last);
			gradedCards.pop();
			reviewed -= 1;
			await loadQueue();
			const undone: SessionCard | undefined = queue.find((item) => item.id === last);
			if (undone) queue = [undone, ...queue.filter((item) => item.id !== last)];
		} catch (reason) {
			error = String(reason);
		}
		saving = false;
	}

	async function suspend(): Promise<void> {
		if (!card || saving) return;
		const id: number = card.id;
		saving = true;
		error = null;
		try {
			await setSuspended(id, true);
			await loadQueue();
			toast('Card suspended');
		} catch (reason) {
			error = String(reason);
		}
		saving = false;
	}

	async function explain(): Promise<void> {
		if (!card || explaining) return;
		if (explanation !== null) {
			explanation = null;
			return;
		}
		const asked: number = card.id;
		explaining = true;
		error = null;
		try {
			const text: string = await explainCard(asked);
			if (card?.id === asked) explanation = text;
		} catch (reason) {
			error = String(reason);
		}
		if (card?.id === asked) explaining = false;
	}

	function startEditing(): void {
		if (!card) return;
		editQuestion = card.question;
		editAnswer = card.answer;
		editing = true;
	}

	async function saveEdit(): Promise<void> {
		if (!card || editQuestion.trim() === '' || editAnswer.trim() === '') return;
		error = null;
		try {
			await editCard(card.id, editQuestion, editAnswer);
			card.question = editQuestion;
			card.answer = editAnswer;
			editing = false;
		} catch (reason) {
			error = String(reason);
		}
	}

	function onkeydown(event: KeyboardEvent): void {
		if (editing) {
			if (event.key === 'Escape') editing = false;
			if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) saveEdit();
			return;
		}
		if (isTyping(event)) return;
		if (event.key === 'z') {
			undo();
		} else if (event.key === 'Escape') {
			goto('/');
		} else if (event.key === ' ') {
			event.preventDefault();
			if (card) revealed = true;
		} else if (revealed && card) {
			if (event.key === 'c' && card.excerpt) showContext = !showContext;
			if (event.key === 'x') explain();
			if (event.key === 's') suspend();
			if (event.key === 'e') {
				event.preventDefault();
				startEditing();
			}
			const position: number = ['1', '2', '3', '4'].indexOf(event.key);
			if (position >= 0) grade(card.grades[position].grade);
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="flex h-screen flex-col bg-background">
	<header class="flex items-center justify-between gap-4 px-6 pt-8 pb-4">
		<Button href="/" variant="ghost" class="gap-2 text-foreground-secondary">
			<X size={15} />End session <span class="font-mono text-xs text-muted-foreground">Esc</span>
		</Button>
		<div class="flex max-w-80 flex-1 flex-col items-center gap-2">
			{#if total > 0}
				<span class="font-mono text-xs text-muted-foreground">
					{Math.min(reviewed + 1, total)} of {total}
				</span>
				<Progress value={reviewed} max={total} class="h-0.75" />
			{/if}
		</div>
		<Button
			variant="ghost"
			onclick={undo}
			disabled={gradedCards.length === 0 || saving}
			class="gap-2 text-foreground-secondary"
		>
			<Undo2 size={15} />Undo <span class="font-mono text-xs text-muted-foreground">Z</span>
		</Button>
	</header>

	{#if error}
		<div class="mx-auto w-full max-w-170 px-6"><Notice>{error}</Notice></div>
	{/if}

	{#if card}
		<main class="flex min-h-0 flex-1 items-center justify-center overflow-y-auto px-6 py-4">
			<article
				class="flex w-full max-w-170 flex-col gap-6 rounded-card border border-card-border bg-card px-12 py-11 shadow-card"
			>
				{#if card.source_title}
					<span class="flex items-center gap-2 text-xs text-muted-foreground">
						<FileText size={13} />{card.source_title}
					</span>
				{/if}
				{#if editing}
					<Input bind:value={editQuestion} aria-label="Question" class="h-9 font-serif text-base" />
					<Textarea bind:value={editAnswer} aria-label="Answer" class="font-serif text-base" />
					<div class="flex gap-2">
						<Button onclick={saveEdit} class="gap-2">
							Save
							<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">⌘↵</Kbd>
						</Button>
						<Button variant="ghost" onclick={() => (editing = false)}>Cancel</Button>
					</div>
				{:else}
					<h1 class="font-serif text-3xl leading-tight font-normal text-pretty">
						<CardQuestion kind={card.kind} question={card.question} {revealed} />
					</h1>
				{/if}

				{#if revealed && !editing}
					<div class="flex animate-reveal flex-col gap-5">
						<div class="h-px bg-border"></div>
						{#if card.kind !== 'cloze'}
							<p class="font-serif text-xl leading-normal text-pretty text-card-answer">
								{card.answer}
							</p>
						{/if}
						{#if explaining}
							<div class="flex flex-col gap-2 rounded-lg bg-muted px-4 py-3.5">
								<span class="text-xs font-medium text-muted-foreground">Explanation</span>
								<Skeleton class="h-4 w-full" />
								<Skeleton class="h-4 w-2/3" />
							</div>
						{:else if explanation !== null}
							<div class="flex animate-reveal flex-col gap-1.5 rounded-lg bg-muted px-4 py-3.5">
								<span class="text-xs font-medium text-muted-foreground">Explanation</span>
								<p class="text-sm leading-normal text-card-answer">{explanation}</p>
							</div>
						{/if}
						{#if showContext && card.excerpt}
							<div class="flex animate-reveal flex-col gap-1.5 rounded-lg border bg-secondary px-4 py-3.5">
								<span class="text-xs font-medium text-muted-foreground">
									From {card.source_title}
								</span>
								<p class="font-serif text-base leading-normal text-card-answer">{card.excerpt}</p>
							</div>
						{/if}
						<div class="-ml-2.5 flex gap-1">
							<Button
								variant="ghost"
								size="sm"
								onclick={explain}
								disabled={explaining}
								class="gap-1.5 text-xs text-foreground-secondary"
							>
								Explain <span class="font-mono text-muted-foreground">X</span>
							</Button>
							<Button
								variant="ghost"
								size="sm"
								onclick={suspend}
								class="gap-1.5 text-xs text-foreground-secondary"
							>
								Suspend <span class="font-mono text-muted-foreground">S</span>
							</Button>
							<Button
								variant="ghost"
								size="sm"
								onclick={startEditing}
								class="gap-1.5 text-xs text-foreground-secondary"
							>
								Edit card <span class="font-mono text-muted-foreground">E</span>
							</Button>
							{#if card.excerpt}
								<Button
									variant="ghost"
									size="sm"
									onclick={() => (showContext = !showContext)}
									class="gap-1.5 text-xs text-foreground-secondary"
								>
									Show context <span class="font-mono text-muted-foreground">C</span>
								</Button>
							{/if}
						</div>
					</div>
				{/if}
			</article>
		</main>

		<footer class="flex min-h-23 flex-col items-center gap-3.5 px-6 pt-2 pb-8">
			{#if revealed}
				<div class="grid w-full max-w-140 animate-grades grid-cols-4 gap-2.5">
					{#each card.grades as preview, index (preview.grade)}
						<Button
							variant={preview.grade === 'good' ? 'default' : 'secondary'}
							onclick={() => grade(preview.grade)}
							class={cn(
								'h-auto flex-col gap-1 rounded-xl pt-3 pb-2.5',
								preview.grade === 'good' ? 'shadow-primary' : 'border-input'
							)}
						>
							{GRADE_LABELS[preview.grade]}
							<span class="font-mono text-xs opacity-80">{index + 1} · {preview.interval}</span>
						</Button>
					{/each}
				</div>
			{:else}
				<Button
					variant="secondary"
					onclick={() => (revealed = true)}
					class="h-auto w-full max-w-90 gap-3 rounded-xl border-input px-5 py-4 text-base"
				>
					Show answer <Kbd class="font-mono">Space</Kbd>
				</Button>
			{/if}
		</footer>
	{:else if loaded}
		<main class="flex flex-1 animate-reveal flex-col items-center justify-center gap-4">
			<h1 class="font-serif text-3xl font-normal">That is everything for now.</h1>
			{#if nextDue}
				<p class="text-base text-muted-foreground">
					Next review: {new Date(nextDue).toLocaleString('en-GB', {
						weekday: 'long',
						hour: '2-digit',
						minute: '2-digit'
					})}
				</p>
			{/if}
			<Button href="/" class="h-auto rounded-xl px-5 py-3 shadow-primary">Back to Today</Button>
		</main>
	{/if}
</div>
