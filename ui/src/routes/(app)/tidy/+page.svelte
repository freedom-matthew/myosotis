<script lang="ts">
	import { Check } from '@lucide/svelte';
	import { toast } from 'svelte-sonner';
	import { markNotDuplicates, mergeGroup, tidyGroups, type TidyGroup, type TidyMember } from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import { isTyping } from '#lib/keys.js';

	let groups: TidyGroup[] = $state<TidyGroup[]>([]);
	let loaded: boolean = $state(false);
	let openIndex: number = $state(0);
	let editing: boolean = $state(false);
	let question: string = $state('');
	let answer: string = $state('');
	let error: string | null = $state(null);

	const group: TidyGroup | undefined = $derived(groups[openIndex]);
	const memberCount: number = $derived(
		groups.reduce((total, item) => total + item.members.length, 0)
	);

	function open(index: number): void {
		openIndex = index;
		editing = false;
		question = groups[index]?.suggested_question ?? '';
		answer = groups[index]?.suggested_answer ?? '';
	}

	async function load(): Promise<void> {
		try {
			groups = await tidyGroups();
			loaded = true;
			open(Math.min(openIndex, Math.max(groups.length - 1, 0)));
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

	function merge(): void {
		if (!group) return;
		const members: TidyMember[] = group.members;
		const keeper: TidyMember | undefined = members.find((member) => member.keeps_history);
		if (!keeper || question.trim() === '' || answer.trim() === '') return;
		const others: number[] = members
			.filter((member) => member.id !== keeper.id)
			.map((member) => member.id);
		act(() => mergeGroup(keeper.id, others, question, answer));
		toast('Cards merged');
	}

	function notDuplicates(): void {
		if (!group) return;
		const ids: number[] = group.members.map((member) => member.id);
		act(() => markNotDuplicates(ids));
	}

	$effect(() => {
		load();
	});

	function onkeydown(event: KeyboardEvent): void {
		if (editing && event.key === 'Escape') editing = false;
		if (isTyping(event) || event.metaKey || event.ctrlKey || !group) return;
		if (event.key === 'm') merge();
		else if (event.key === 'n') notDuplicates();
		else if (event.key === 'j') open(Math.min(openIndex + 1, groups.length - 1));
		else if (event.key === 'k') open(Math.max(openIndex - 1, 0));
		else if (event.key === 'e') {
			event.preventDefault();
			editing = true;
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="mx-auto flex max-w-220 flex-col gap-7 px-10 pt-9 pb-10">
	<header class="flex flex-col gap-1">
		<a href="/library" class="text-xs text-muted-foreground hover:text-foreground">Library</a>
		<h1 class="font-serif text-3xl font-normal">Tidy up</h1>
		{#if loaded && groups.length > 0}
			<p class="text-sm text-muted-foreground">
				{groups.length}
				{groups.length === 1 ? 'group' : 'groups'} of similar cards found, {memberCount} cards in all.
				Nothing changes until you confirm.
			</p>
		{/if}
	</header>

	{#if error}
		<Notice>{error}</Notice>
	{/if}

	{#each groups as item, index}
		{#if index === openIndex}
			<section class="flex flex-col gap-4 rounded-2xl border border-primary bg-card p-6">
				<div class="grid auto-cols-fr grid-flow-col gap-4">
					{#each item.members as member (member.id)}
						<div class="flex flex-col gap-1.5">
							<p class="font-serif text-base">{member.question}</p>
							<p class="text-sm text-card-answer">{member.answer}</p>
							<span class="text-xs text-muted-foreground">
								{member.review_count}
								{member.review_count === 1 ? 'review' : 'reviews'}
							</span>
							{#if member.keeps_history}
								<span
									class="self-start rounded-full bg-primary-tint px-2.5 py-0.5 text-xs text-primary-strong"
								>
									Keeps its review history
								</span>
							{/if}
						</div>
					{/each}
				</div>
				<div class="flex flex-col gap-2 rounded-lg bg-primary-soft p-4">
					<span class="text-xs font-medium text-primary-strong">Suggested merged card</span>
					{#if editing}
						<Input bind:value={question} aria-label="Question" class="h-9 bg-card" />
						<Textarea bind:value={answer} aria-label="Answer" class="bg-card" />
					{:else}
						<p class="font-serif text-lg">{question}</p>
						<p class="text-sm text-card-answer">{answer}</p>
					{/if}
				</div>
				<div class="flex flex-wrap gap-1">
					<Button onclick={merge} class="gap-2">
						Merge <Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">M</Kbd>
					</Button>
					<Button variant="ghost" onclick={() => (editing = !editing)} class="gap-2">
						Edit merged card <Kbd class="font-mono">E</Kbd>
					</Button>
					<Button variant="ghost" onclick={notDuplicates} class="gap-2">
						Not duplicates <Kbd class="font-mono">N</Kbd>
					</Button>
				</div>
			</section>
		{:else}
			<button
				type="button"
				onclick={() => open(index)}
				class="flex items-center justify-between gap-4 rounded-xl border bg-card px-4 py-3 text-left text-sm hover:border-input"
			>
				<span class="truncate font-serif text-base">{item.suggested_question}</span>
				<span class="shrink-0 text-xs text-muted-foreground">{item.members.length} cards</span>
			</button>
		{/if}
	{/each}

	{#if loaded && groups.length === 0}
		<div class="flex flex-col items-center gap-3 py-16">
			<Check size={20} strokeWidth={1.8} class="text-primary" />
			<h2 class="font-serif text-3xl font-normal">All tidy.</h2>
		</div>
	{/if}
</div>
