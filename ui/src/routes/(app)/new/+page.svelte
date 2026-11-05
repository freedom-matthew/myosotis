<script lang="ts">
	import { goto } from '$app/navigation';
	import { startImport, type Depth } from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';

	type DepthOption = { value: Depth; label: string; cards: number };

	const DEPTHS: DepthOption[] = [
		{ value: 'essentials', label: 'Essentials', cards: 6 },
		{ value: 'balanced', label: 'Balanced', cards: 14 },
		{ value: 'thorough', label: 'Thorough', cards: 30 }
	];

	let title: string = $state('');
	let text: string = $state('');
	let focus: string = $state('');
	let depth: Depth = $state('balanced');
	let error: string | null = $state(null);
	let starting: boolean = $state(false);

	const words: number = $derived(text.trim() === '' ? 0 : text.trim().split(/\s+/).length);
	const canGenerate: boolean = $derived(title.trim() !== '' && text.trim() !== '' && !starting);
	const expected: number = $derived(
		DEPTHS.find((option) => option.value === depth)?.cards ?? 14
	);

	async function generate(): Promise<void> {
		if (!canGenerate) return;
		starting = true;
		try {
			await startImport({
				title: title.trim(),
				text,
				depth,
				focus: focus.trim() === '' ? null : focus.trim()
			});
			await goto('/inbox');
		} catch (reason) {
			error = String(reason);
			starting = false;
		}
	}

	function onkeydown(event: KeyboardEvent): void {
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			generate();
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="flex flex-col gap-7 px-10 pt-9 pb-10">
	<header class="flex flex-col gap-2">
		<h1 class="font-serif text-3xl font-normal">New source</h1>
		<p class="text-sm text-muted-foreground">
			Paste anything worth keeping. Cards land in your Inbox for a quick check.
		</p>
	</header>

	{#if error}
		<Notice>{error}</Notice>
	{/if}

	<div class="grid grid-cols-[1fr_18rem] gap-7">
		<div class="flex flex-col gap-5">
			<div class="flex flex-col gap-2">
				<Label for="title">Title</Label>
				<Input id="title" bind:value={title} class="h-9 bg-card" />
			</div>
			<div class="flex flex-col gap-2">
				<div class="flex items-center justify-between">
					<Label for="text">Text</Label>
					<span class="font-mono text-xs text-muted-foreground">{words} words</span>
				</div>
				<Textarea id="text" bind:value={text} class="min-h-96 bg-card font-serif text-base" />
			</div>
		</div>

		<aside class="flex flex-col gap-5 self-start rounded-2xl border bg-card p-5">
			<div class="flex flex-col gap-2">
				<span class="text-sm font-medium">How much to extract</span>
				<ToggleGroup.Root
					type="single"
					value={depth}
					onValueChange={(value: string) => {
						if (value) depth = value as Depth;
					}}
					class="w-full"
				>
					{#each DEPTHS as option (option.value)}
						<ToggleGroup.Item value={option.value} class="flex-1 text-xs">
							{option.label}
						</ToggleGroup.Item>
					{/each}
				</ToggleGroup.Root>
			</div>
			<div class="flex flex-col gap-2">
				<Label for="focus">
					Focus on <span class="font-normal text-muted-foreground">(optional)</span>
				</Label>
				<Input id="focus" bind:value={focus} class="h-9" />
			</div>
			<div class="flex flex-col gap-2">
				<Button
					onclick={generate}
					disabled={!canGenerate}
					class="h-auto gap-3 rounded-xl px-5 py-3 shadow-primary"
				>
					Generate cards
					<Kbd class="bg-primary-foreground/20 font-mono text-primary-foreground">⌘↵</Kbd>
				</Button>
				<span class="text-center text-xs text-muted-foreground">
					Claude Sonnet · about {expected} cards
				</span>
			</div>
		</aside>
	</div>
</div>
