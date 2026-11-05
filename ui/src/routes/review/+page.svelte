<script lang="ts">
	import { goto } from '$app/navigation';
	import { FileText, X } from '@lucide/svelte';
	import { gradeCard, reviewSession, type Grade, type SessionCard } from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Progress } from '#lib/components/ui/progress/index.js';
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

	const card: SessionCard | undefined = $derived(queue[0]);
	const total: number = $derived(reviewed + queue.length);

	async function loadQueue(): Promise<void> {
		queue = await reviewSession();
		revealed = false;
		showContext = false;
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
			reviewed += 1;
			await loadQueue();
		} catch (reason) {
			error = String(reason);
		}
		saving = false;
	}

	function onkeydown(event: KeyboardEvent): void {
		if (isTyping(event)) return;
		if (event.key === 'Escape') {
			goto('/');
		} else if (event.key === ' ') {
			event.preventDefault();
			if (card) revealed = true;
		} else if (revealed && card) {
			if (event.key === 'c' && card.excerpt) showContext = !showContext;
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
		<div class="w-32"></div>
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
				<h1 class="font-serif text-3xl leading-tight font-normal text-pretty">{card.question}</h1>

				{#if revealed}
					<div class="flex animate-reveal flex-col gap-5">
						<div class="h-px bg-border"></div>
						<p class="font-serif text-xl leading-normal text-pretty text-card-answer">
							{card.answer}
						</p>
						{#if showContext && card.excerpt}
							<div class="flex animate-reveal flex-col gap-1.5 rounded-lg border bg-secondary px-4 py-3.5">
								<span class="text-xs font-medium text-muted-foreground">
									From {card.source_title}
								</span>
								<p class="font-serif text-base leading-normal text-card-answer">{card.excerpt}</p>
							</div>
						{/if}
						{#if card.excerpt}
							<div class="-ml-2.5">
								<Button
									variant="ghost"
									size="sm"
									onclick={() => (showContext = !showContext)}
									class="gap-1.5 text-xs text-foreground-secondary"
								>
									Show context <span class="font-mono text-muted-foreground">C</span>
								</Button>
							</div>
						{/if}
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
			<Button href="/" class="h-auto rounded-xl px-5 py-3 shadow-primary">Back to Today</Button>
		</main>
	{/if}
</div>
