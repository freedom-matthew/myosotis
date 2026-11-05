<script lang="ts">
	import { goto } from '$app/navigation';
	import { FileUp } from '@lucide/svelte';
	import { startImport, type Depth } from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Label } from '#lib/components/ui/label/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import { cn } from '#lib/utils.js';

	type DepthOption = { value: Depth; label: string; cards: number };
	type InputMode = 'paste' | 'file';

	const SUPPORTED_FILE: RegExp = /\.(txt|md)$/i;

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
	let mode: InputMode = $state('paste');
	let dragging: boolean = $state(false);

	const words: number = $derived(text.trim() === '' ? 0 : text.trim().split(/\s+/).length);
	const canGenerate: boolean = $derived(title.trim() !== '' && text.trim() !== '' && !starting);
	const expected: number = $derived(
		DEPTHS.find((option) => option.value === depth)?.cards ?? 14
	);

	/** Loads a file into the form, so the text can be checked before generating. */
	async function openFile(file: File): Promise<void> {
		error = null;
		if (!SUPPORTED_FILE.test(file.name)) {
			error = 'Only .txt and .md files can be opened for now.';
			return;
		}
		try {
			text = await file.text();
			if (title.trim() === '') title = file.name.replace(SUPPORTED_FILE, '');
			mode = 'paste';
		} catch (reason) {
			error = String(reason);
		}
	}

	function onFileChosen(event: Event): void {
		const input: HTMLInputElement = event.currentTarget as HTMLInputElement;
		const file: File | undefined = input.files?.[0];
		if (file) openFile(file);
		input.value = '';
	}

	function onDrop(event: DragEvent): void {
		event.preventDefault();
		dragging = false;
		const file: File | undefined = event.dataTransfer?.files[0];
		if (file) openFile(file);
	}

	function onDragOver(event: DragEvent): void {
		event.preventDefault();
		dragging = true;
	}

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
			Paste or open anything worth keeping. Cards land in your Inbox for a quick check.
		</p>
	</header>

	{#if error}
		<Notice>{error}</Notice>
	{/if}

	<ToggleGroup.Root
		type="single"
		value={mode}
		onValueChange={(value: string) => {
			if (value) mode = value as InputMode;
		}}
		class="self-start"
	>
		<ToggleGroup.Item value="paste">Paste text</ToggleGroup.Item>
		<ToggleGroup.Item value="file">Open file</ToggleGroup.Item>
	</ToggleGroup.Root>

	<div class="grid grid-cols-[1fr_18rem] gap-7">
		<div class="flex flex-col gap-5">
			<div class="flex flex-col gap-2">
				<Label for="title">Title</Label>
				<Input id="title" bind:value={title} class="h-9 bg-card" />
			</div>
			{#if mode === 'paste'}
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between">
						<Label for="text">Text</Label>
						<span class="font-mono text-xs text-muted-foreground">{words} words</span>
					</div>
					<Textarea id="text" bind:value={text} class="min-h-96 bg-card font-serif text-base" />
				</div>
			{:else}
				<label
					ondrop={onDrop}
					ondragover={onDragOver}
					ondragleave={() => (dragging = false)}
					class={cn(
						'flex min-h-96 flex-col items-center justify-center gap-3 rounded-2xl border-2 border-dashed bg-card p-6 text-center has-focus-visible:ring-2 has-focus-visible:ring-ring',
						dragging ? 'border-primary bg-primary-soft' : 'border-input'
					)}
				>
					<FileUp size={20} strokeWidth={1.8} class="text-muted-foreground" />
					<span class="text-base">Drop a file here</span>
					<span class="font-mono text-xs text-muted-foreground">.txt · .md</span>
					<span
						class="rounded-lg border border-input bg-secondary px-3 py-2 text-sm text-secondary-foreground"
					>
						Choose file…
					</span>
					<input type="file" accept=".txt,.md" onchange={onFileChosen} class="sr-only" />
				</label>
			{/if}
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
					Claude Opus · about {expected} cards
				</span>
			</div>
		</aside>
	</div>
</div>
