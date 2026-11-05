<script lang="ts">
	import { Check } from '@lucide/svelte';
	import {
		acceptAllDrafts,
		acceptDraft,
		deleteDraft,
		discardImport,
		editDraft,
		inbox,
		retryImport,
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
	import { cn } from '#lib/utils.js';

	const POLL_MS: number = 2000;

	let imports: InboxImport[] = $state([]);
	let loaded: boolean = $state(false);
	let error: string | null = $state(null);
	let focusedIndex: number = $state(0);
	let editing: boolean = $state(false);
	let editQuestion: string = $state('');
	let editAnswer: string = $state('');

	const failed: InboxImport[] = $derived(imports.filter((item) => item.status === 'failed'));
	const generating: InboxImport[] = $derived(
		imports.filter((item) => item.status === 'generating')
	);
	const current: InboxImport | undefined = $derived(
		imports.find((item) => item.status === 'in_inbox')
	);
	const drafts: DraftCard[] = $derived(current?.drafts ?? []);
	const focused: DraftCard | undefined = $derived(drafts[focusedIndex]);

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
			focusedIndex = Math.min(focusedIndex, Math.max((current?.drafts.length ?? 1) - 1, 0));
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
		else if (event.key === 'k') focusedIndex = Math.max(focusedIndex - 1, 0);
		else if (event.key === 'a') act(() => acceptDraft(card.id));
		else if (event.key === 'A' && current) {
			const id: number = current.id;
			act(() => acceptAllDrafts(id));
		} else if (event.key === 'd') act(() => deleteDraft(card.id));
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

	{#each generating as item (item.id)}
		<div class="flex flex-col gap-3 rounded-2xl border bg-card p-5">
			<span class="text-sm text-muted-foreground">Generating cards from “{item.title}”…</span>
			<Skeleton class="h-6 w-3/4" />
			<Skeleton class="h-6 w-1/2" />
		</div>
	{/each}

	{#if current}
		<header class="flex items-end justify-between gap-6">
			<div class="flex flex-col gap-1">
				<span class="text-xs text-muted-foreground">Inbox</span>
				<h1 class="font-serif text-3xl font-normal">{current.title}</h1>
				<span class="text-sm text-muted-foreground">{drafts.length} to check</span>
			</div>
			<Button onclick={() => act(() => acceptAllDrafts(current.id))} class="gap-2">
				Accept all remaining
				<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">⇧A</Kbd>
			</Button>
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
							{:else}
								<h2 class="font-serif text-lg font-normal">{draft.question}</h2>
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
										onclick={() => act(() => deleteDraft(draft.id))}
										class="gap-2"
									>
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
							class={cn('rounded-xl border bg-card p-4 text-left transition-colors hover:border-input')}
						>
							<span class="block font-serif text-lg">{draft.question}</span>
							<span class="block truncate text-sm text-muted-foreground">{draft.answer}</span>
						</button>
					{/if}
				{/each}
			</section>
		</div>

		<footer class="flex gap-4 text-xs text-muted-foreground">
			<span><Kbd class="font-mono">J</Kbd> <Kbd class="font-mono">K</Kbd> move</span>
			<span><Kbd class="font-mono">A</Kbd> accept</span>
			<span><Kbd class="font-mono">E</Kbd> edit</span>
			<span><Kbd class="font-mono">D</Kbd> delete</span>
		</footer>
	{:else if loaded && failed.length === 0 && generating.length === 0}
		<div class="flex flex-1 flex-col items-center justify-center gap-4">
			<h1 class="font-serif text-3xl font-normal">Nothing to check.</h1>
			<Button href="/new" variant="secondary">New source</Button>
		</div>
	{/if}
</div>
