<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		claudeStatus,
		todaySummary,
		type ClaudeStatus,
		type RecentImport,
		type TodaySummary
	} from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { isTyping } from '#lib/keys.js';
	import { cn } from '#lib/utils.js';

	const BAR_HEIGHTS: string[] = ['h-1', 'h-2', 'h-3', 'h-4', 'h-6', 'h-8', 'h-10', 'h-12'];

	let summary: TodaySummary | null = $state<TodaySummary | null>(null);
	let error: string | null = $state(null);
	let claudeReady: boolean = $state(true);

	const due: number = $derived(summary ? summary.due_reviews + summary.due_new : 0);
	const dateLabel: string = new Date().toLocaleDateString('en-GB', {
		weekday: 'long',
		day: 'numeric',
		month: 'long'
	});

	$effect(() => {
		todaySummary()
			.then((result: TodaySummary) => (summary = result))
			.catch((reason: unknown) => (error = String(reason)));
		claudeStatus().then((status: ClaudeStatus) => (claudeReady = status.logged_in));
	});

	function barHeight(count: number, forecast: number[]): string {
		const highest: number = Math.max(...forecast, 1);
		const step: number = Math.ceil((count / highest) * (BAR_HEIGHTS.length - 1));
		return BAR_HEIGHTS[step];
	}

	function relativeDate(iso: string): string {
		const days: number = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
		if (days <= 0) return 'today';
		return days === 1 ? '1 day' : `${days} days`;
	}

	function nextDueLabel(iso: string): string {
		return new Date(iso).toLocaleString('en-GB', {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function importStatus(item: RecentImport): string {
		if (item.status === 'generating') return 'Generating';
		if (item.status === 'failed') return 'Failed';
		if (item.draft_count > 0) return `${item.draft_count} in Inbox`;
		return item.card_count === 1 ? '1 card' : `${item.card_count} cards`;
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === ' ' && due > 0 && !isTyping(event)) {
			event.preventDefault();
			goto('/review');
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="mx-auto flex max-w-220 flex-col gap-10 p-12 pt-16">
	{#if error}
		<Notice>{error}</Notice>
	{/if}

	{#if !claudeReady}
		<p class="rounded-lg bg-muted px-4 py-3 text-sm text-muted-foreground">
			Claude Code isn't connected, so new sources can't become cards.
			<a href="/settings" class="text-primary underline">Check Settings</a>
		</p>
	{/if}

	{#if summary}
		<section class="flex flex-col gap-3.5">
			<span class="text-sm text-muted-foreground">{dateLabel}</span>
			{#if due > 0}
				<h1 class="font-serif text-5xl leading-[1.1] font-normal">
					{due === 1 ? '1 card is' : `${due} cards are`} due today.
				</h1>
				<p class="text-base text-muted-foreground">
					About {summary.minutes}
					{summary.minutes === 1 ? 'minute' : 'minutes'} · {summary.due_reviews}
					{summary.due_reviews === 1 ? 'review' : 'reviews'} and {summary.due_new} new
				</p>
				<div class="mt-2.5 flex flex-wrap items-center gap-4">
					<Button href="/review" class="h-auto gap-3.5 rounded-xl px-5 py-3 text-base shadow-primary">
						Start review
						<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">Space</Kbd>
					</Button>
					{#if summary.due_reviews > 0 && summary.due_new > 0}
						<a href="/review?skip_new=1" class="text-sm text-foreground-secondary underline">
							Only reviews, skip new
						</a>
					{/if}
				</div>
			{:else}
				<h1 class="font-serif text-5xl leading-[1.1] font-normal">Nothing due today.</h1>
				{#if summary.next_due}
					<p class="text-base text-muted-foreground">
						Next card: {nextDueLabel(summary.next_due)}
					</p>
				{:else}
					<p class="text-base text-muted-foreground">
						Start with something worth keeping: an article, a chapter, your notes.
					</p>
				{/if}
			{/if}
		</section>

		<section class="grid grid-cols-3 gap-3.5">
			<div class="flex flex-col gap-2 rounded-2xl border bg-card p-5">
				<span class="text-xs text-muted-foreground">Next 7 days</span>
				<div
					class="flex h-12 items-end gap-1.5"
					role="img"
					aria-label={`Cards due: ${summary.forecast.join(', ')}`}
				>
					{#each summary.forecast as count, day}
						<span
							class={cn(
								'flex-1 rounded-sm',
								barHeight(count, summary.forecast),
								day === 0 ? 'bg-chart-1' : 'bg-chart-2'
							)}
						></span>
					{/each}
				</div>
			</div>
			<div class="flex flex-col gap-2 rounded-2xl border bg-card p-5">
				<span class="text-xs text-muted-foreground">Retention, 30 days</span>
				<span class="font-mono text-2xl font-medium">
					{summary.retention_percent === null ? '—' : `${summary.retention_percent}%`}
				</span>
			</div>
			<a
				href="/library?filter=slipping"
				class="flex flex-col gap-2 rounded-2xl border bg-card p-5 transition-colors hover:border-input"
			>
				<span class="text-xs text-muted-foreground">Keeps slipping</span>
				<span class="text-2xl font-medium">
					{summary.slipping_count === 1 ? '1 card' : `${summary.slipping_count} cards`}
				</span>
				<span class="text-xs text-muted-foreground">Rewrite or suspend them</span>
			</a>
		</section>

		{#if summary.recent_imports.length > 0}
			<section class="flex flex-col gap-3">
				<h2 class="text-sm font-medium">In progress</h2>
				<div class="flex flex-col divide-y rounded-2xl border bg-card">
					{#each summary.recent_imports as item (item.id)}
						<div class="flex items-center gap-4 px-4 py-3 text-sm">
							<span class="flex-1 truncate font-serif text-base">{item.title}</span>
							<span class="text-muted-foreground">{importStatus(item)}</span>
							<span class="font-mono text-xs text-muted-foreground">
								{relativeDate(item.created_at)}
							</span>
						</div>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</div>
