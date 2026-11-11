<script lang="ts">
	import { Check } from '@lucide/svelte';
	import { KIND_LABELS } from '#lib/card-kind.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { toast } from 'svelte-sonner';
	import {
		acceptAllDrafts,
		acceptDraft,
		deleteDraft,
		discardImport,
		editDraft,
		inbox,
		keepBothCards,
		mergeSourceIntoExisting,
		retryImport,
		rewriteDraft,
		stopImport,
		type DraftCard,
		type InboxImport
	} from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import { isTyping } from '#lib/keys.js';

	const POLL_MS: number = 2000;

	let imports: InboxImport[] = $state([]);
	let loaded: boolean = $state(false);
	let error: string | null = $state(null);
	let focusedIndex: number = $state(0);
	let editing: boolean = $state(false);
	let editQuestion: string = $state('');
	let editAnswer: string = $state('');
	let rewriting: boolean = $state(false);

	const failed: InboxImport[] = $derived(imports.filter((item) => item.status === 'failed'));
	const generating: InboxImport[] = $derived(
		imports.filter((item) => item.status === 'generating')
	);
	/** The source being checked: cards can arrive while later parts are still generating. */
	const current: InboxImport | undefined = $derived(
		imports.find((item) => item.drafts.length > 0)
	);
	const drafts: DraftCard[] = $derived(current?.drafts ?? []);
	const focused: DraftCard | undefined = $derived(drafts[focusedIndex]);
	const duplicates: number = $derived(drafts.filter((draft) => draft.duplicate).length);

	const summary: string = $derived.by(() => {
		if (!current) return '';
		const parts: string[] = [`${drafts.length} to check`];
		if (duplicates > 0) {
			parts.push(duplicates === 1 ? '1 possible duplicate' : `${duplicates} possible duplicates`);
		}
		if (current.status === 'generating') {
			parts.push(`still generating, part ${current.parts_done + 1} of ${current.parts_total}`);
		}
		return parts.join(' · ');
	});

	/** The source text split around the focused card's passage, when it can be found. */
	const passage: { before: string; match: string; after: string } | null = $derived.by(() => {
		const text: string | null | undefined = current?.text;
		const excerpt: string | null | undefined = focused?.excerpt;
		if (!text || !excerpt) return null;
		const start: number = text.indexOf(excerpt);
		if (start < 0) return null;
		return {
			before: text.slice(0, start),
			match: excerpt,
			after: text.slice(start + excerpt.length)
		};
	});

	async function refresh(): Promise<void> {
		try {
			imports = await inbox();
			loaded = true;
			const count: number = imports.find((item) => item.drafts.length > 0)?.drafts.length ?? 1;
			focusedIndex = Math.min(focusedIndex, Math.max(count - 1, 0));
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
		await refresh();
	}

	function startEditing(): void {
		if (!focused) return;
		editQuestion = focused.question;
		editAnswer = focused.answer;
		editing = true;
	}

	async function saveEdit(): Promise<void> {
		if (!focused) return;
		const card: DraftCard = focused;
		editing = false;
		await act(() => editDraft(card.id, editQuestion, editAnswer));
	}

	async function rewrite(card: DraftCard): Promise<void> {
		rewriting = true;
		await act(() => rewriteDraft(card.id));
		rewriting = false;
	}

	function deleteCard(card: DraftCard): void {
		act(() => deleteDraft(card.id));
		toast('Card deleted');
	}

	$effect(() => {
		refresh();
		const timer: ReturnType<typeof setInterval> = setInterval(() => {
			if (generating.length > 0) refresh();
		}, POLL_MS);
		return () => clearInterval(timer);
	});

	function onkeydown(event: KeyboardEvent): void {
		if (editing) {
			if (event.key === 'Escape') editing = false;
			if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) saveEdit();
			return;
		}
		if (isTyping(event) || event.metaKey || event.ctrlKey || !focused) return;
		const card: DraftCard = focused;
		if (event.key === 'j') focusedIndex = Math.min(focusedIndex + 1, drafts.length - 1);
		else if (event.key === 'k' && !card.duplicate) focusedIndex = Math.max(focusedIndex - 1, 0);
		else if (event.key === 'k') act(() => keepBothCards(card.id));
		else if (event.key === 's' && card.duplicate) deleteCard(card);
		else if (event.key === 'm' && card.duplicate) act(() => mergeSourceIntoExisting(card.id));
		else if (event.key === 'a' && !card.duplicate) act(() => acceptDraft(card.id));
		else if (event.key === 'A' && current) {
			const id: number = current.id;
			act(() => acceptAllDrafts(id));
		} else if (event.key === 'd') deleteCard(card);
		else if (event.key === 'r' && !rewriting) rewrite(card);
		else if (event.key === 'e') {
			event.preventDefault();
			startEditing();
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="flex h-full flex-col gap-6 px-10 pt-9 pb-6">
	{#if error}
		<Notice>{error}</Notice>
	{/if}

	{#each failed as item (item.id)}
		<Notice>
			<span>“{item.title}”: {item.error}</span>
			<span class="flex shrink-0 gap-2">
				<Button variant="outline" onclick={() => act(() => discardImport(item.id))}>Discard</Button>
				<Button onclick={() => act(() => retryImport(item.id))}>Retry</Button>
			</span>
		</Notice>
	{/each}

	{#each generating.filter((item) => item.id !== current?.id) as item (item.id)}
		<div class="flex flex-col gap-3 rounded-2xl border bg-card p-5">
			<div class="flex items-center justify-between gap-4">
				<span class="text-sm text-muted-foreground">
					Generating cards from “{item.title}”, part {item.parts_done + 1} of {item.parts_total}…
				</span>
				<Button variant="ghost" onclick={() => act(() => stopImport(item.id))}>Stop</Button>
			</div>
			<Skeleton class="h-6 w-3/4" />
			<Skeleton class="h-6 w-1/2" />
		</div>
	{/each}

	{#if current}
		<header class="flex items-end justify-between gap-6">
			<div class="flex flex-col gap-1">
				<span class="text-xs text-muted-foreground">Inbox</span>
				<h1 class="font-serif text-3xl font-normal">{current.title}</h1>
				<span class="text-sm text-muted-foreground">{summary}</span>
			</div>
			<div class="flex gap-2">
				{#if current.status === 'generating'}
					<Button variant="ghost" onclick={() => act(() => stopImport(current.id))}>Stop</Button>
				{/if}
				<Button onclick={() => act(() => acceptAllDrafts(current.id))} class="gap-2">
					Accept all remaining
					<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">⇧A</Kbd>
				</Button>
			</div>
		</header>

		<div class="grid min-h-0 flex-1 grid-cols-2 gap-6">
			<section class="flex min-h-0 flex-col gap-3 overflow-y-auto rounded-2xl border bg-card p-6">
				<span class="text-xs font-medium tracking-wide text-muted-foreground uppercase">
					Source · kept until this Inbox is done
				</span>
				<p class="font-serif text-base leading-relaxed whitespace-pre-wrap">
					{#if passage}{passage.before}<mark
							class="rounded-sm bg-highlight text-foreground">{passage.match}</mark
						>{passage.after}{:else}{current.text}{/if}
				</p>
			</section>

			<section class="flex min-h-0 flex-col gap-2 overflow-y-auto">
				{#each drafts as draft, index (draft.id)}
					{#if index === focusedIndex}
						<article
							class="flex flex-col gap-3 rounded-xl border border-primary bg-card p-4 ring-2 ring-primary/20"
						>
							{#if editing}
								<Input bind:value={editQuestion} aria-label="Question" class="h-9" />
								<Textarea bind:value={editAnswer} aria-label="Answer" />
								<div class="flex gap-2">
									<Button onclick={saveEdit} class="gap-2">
										Save
										<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">⌘↵</Kbd>
									</Button>
									<Button variant="ghost" onclick={() => (editing = false)}>Cancel</Button>
								</div>
							{:else if draft.duplicate}
								<span class="self-start rounded-full bg-hook px-2.5 py-0.5 text-xs text-hook-foreground">
									Similar card already in your collection
								</span>
								<div class="grid grid-cols-2 gap-3">
									<div class="flex flex-col gap-1">
										<span class="text-xs text-muted-foreground">New · from this source</span>
										<p class="font-serif text-base">
											{draft.question}
										</p>
										<p class="text-sm text-card-answer">{draft.answer}</p>
									</div>
									<div class="flex flex-col gap-1">
										<span class="text-xs text-muted-foreground">
											Existing · {draft.duplicate.source_title ?? 'added by hand'} ·
											{draft.duplicate.review_count} reviews
										</span>
										<p class="font-serif text-base">{draft.duplicate.question}</p>
										<p class="text-sm text-card-answer">{draft.duplicate.answer}</p>
									</div>
								</div>
								<div class="flex flex-wrap gap-1">
									<Button onclick={() => deleteCard(draft)} class="gap-2">
										Skip
										<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">S</Kbd>
									</Button>
									<Button
										variant="ghost"
										onclick={() => act(() => mergeSourceIntoExisting(draft.id))}
										class="gap-2"
									>
										Add source to existing <Kbd class="font-mono">M</Kbd>
									</Button>
									<Button
										variant="ghost"
										onclick={() => act(() => keepBothCards(draft.id))}
										class="gap-2"
									>
										Keep both <Kbd class="font-mono">K</Kbd>
									</Button>
								</div>
							{:else}
								{#if draft.kind !== 'question'}
									<Badge variant="secondary" class="self-start">{KIND_LABELS[draft.kind]}</Badge>
								{/if}
								<h2 class="font-serif text-lg font-normal">
									{draft.question}
								</h2>
								<p class="font-serif text-base text-card-answer">{draft.answer}</p>
								<div class="flex flex-wrap gap-1">
									<Button onclick={() => act(() => acceptDraft(draft.id))} class="gap-2">
										<Check size={14} />Accept
										<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">A</Kbd>
									</Button>
									<Button variant="ghost" onclick={startEditing} class="gap-2">
										Edit <Kbd class="font-mono">E</Kbd>
									</Button>
									<Button
										variant="ghost"
										disabled={rewriting}
										onclick={() => rewrite(draft)}
										class="gap-2"
									>
										{rewriting ? 'Rewriting…' : 'Rewrite'}
										<Kbd class="font-mono">R</Kbd>
									</Button>
									<Button variant="ghost" onclick={() => deleteCard(draft)} class="gap-2">
										Delete <Kbd class="font-mono">D</Kbd>
									</Button>
								</div>
							{/if}
						</article>
					{:else}
						<button
							type="button"
							onclick={() => {
								focusedIndex = index;
								editing = false;
							}}
							class="rounded-xl border bg-card p-4 text-left transition-colors hover:border-input"
						>
							{#if draft.kind !== 'question'}
								<span class="text-xs text-muted-foreground">{KIND_LABELS[draft.kind]}</span>
							{/if}
							<span class="block font-serif text-lg">
								{draft.question}
							</span>
							{#if draft.duplicate}
								<span class="text-xs text-hook-foreground">Possible duplicate</span>
							{:else}
								<span class="block truncate text-sm text-muted-foreground">{draft.answer}</span>
							{/if}
						</button>
					{/if}
				{/each}
				{#if current.status === 'generating'}
					<Skeleton class="h-16 w-full shrink-0 rounded-xl" />
				{/if}
			</section>
		</div>

		<footer class="flex gap-4 text-xs text-muted-foreground">
			<span><Kbd class="font-mono">J</Kbd> <Kbd class="font-mono">K</Kbd> move</span>
			<span><Kbd class="font-mono">A</Kbd> accept</span>
			<span><Kbd class="font-mono">E</Kbd> edit</span>
			<span><Kbd class="font-mono">R</Kbd> ask AI to rewrite</span>
			<span><Kbd class="font-mono">D</Kbd> delete</span>
			{#if duplicates > 0}
				<span>
					<Kbd class="font-mono">S</Kbd>
					<Kbd class="font-mono">M</Kbd>
					<Kbd class="font-mono">K</Kbd> duplicates: skip · add source · keep both
				</span>
			{/if}
		</footer>
	{:else if loaded && failed.length === 0 && generating.length === 0}
		<div class="flex flex-1 items-center justify-center">
			<h1 class="font-serif text-3xl font-normal">Nothing to check.</h1>
		</div>
	{/if}
</div>
