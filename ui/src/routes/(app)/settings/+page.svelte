<script lang="ts">
	import { Minus, Plus } from '@lucide/svelte';
	import { toast } from 'svelte-sonner';
	import {
		backUpNow,
		claudeStatus,
		dataFolder,
		exportCards,
		getSettings,
		listBackups,
		restoreBackup,
		updateSettings,
		type Backup,
		type ClaudeStatus,
		type Settings,
		type Size,
		type Theme
	} from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import { SETTINGS_CHANGED } from '#lib/size.js';
	import { applyTheme } from '#lib/theme.js';

	const NEW_CARDS_STEP: number = 5;

	let settings: Settings | null = $state<Settings | null>(null);
	let claude: ClaudeStatus | null = $state<ClaudeStatus | null>(null);
	let folder: string = $state('');
	let saved: string | null = $state(null);
	let error: string | null = $state(null);
	let backups: Backup[] | null = $state<Backup[] | null>(null);
	let choosing: boolean = $state(false);
	let chosen: Backup | null = $state<Backup | null>(null);
	let confirming: boolean = $state(false);
	let restoring: boolean = $state(false);

	const claudeLabel: string = $derived.by(() => {
		if (!claude) return 'Checking…';
		if (!claude.installed) return 'Claude Code not found';
		if (!claude.logged_in) return 'Not signed in. Run claude in a terminal.';
		return claude.plan ? `Connected · ${claude.plan} plan` : 'Connected';
	});

	$effect(() => {
		getSettings()
			.then((result: Settings) => (settings = result))
			.catch((reason: unknown) => (error = String(reason)));
		claudeStatus().then((result: ClaudeStatus) => (claude = result));
		dataFolder().then((result: string) => (folder = result));
	});

	$effect(() => {
		function follow(event: Event): void {
			settings = (event as CustomEvent<Settings>).detail;
		}
		window.addEventListener(SETTINGS_CHANGED, follow);
		return () => window.removeEventListener(SETTINGS_CHANGED, follow);
	});

	async function save(changes: Partial<Settings>): Promise<void> {
		if (!settings) return;
		error = null;
		try {
			settings = await updateSettings({ ...settings, ...changes });
		} catch (reason) {
			error = String(reason);
		}
	}

	function chooseTheme(value: string): void {
		if (!value) return;
		const theme: Theme = value as Theme;
		applyTheme(theme);
		save({ theme });
	}

	function chooseSize(value: string): void {
		if (value) save({ size: value as Size });
	}

	function changeNewCards(event: Event): void {
		const input: HTMLInputElement = event.currentTarget as HTMLInputElement;
		const count: number = Number.parseInt(input.value, 10);
		if (Number.isInteger(count) && count >= 0) save({ new_cards_per_day: count });
		else if (settings) input.value = String(settings.new_cards_per_day);
	}

	function stepNewCards(change: number): void {
		if (settings) save({ new_cards_per_day: Math.max(0, settings.new_cards_per_day + change) });
	}

	function day(backup: Backup): string {
		return new Date(backup.taken_at).toLocaleDateString('en-GB', {
			weekday: 'long',
			day: 'numeric',
			month: 'long'
		});
	}

	function time(backup: Backup): string {
		return new Date(backup.taken_at).toLocaleTimeString('en-GB', {
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function cards(backup: Backup): string {
		if (backup.card_count === null) return 'unreadable';
		return backup.card_count === 1 ? '1 card' : `${backup.card_count} cards`;
	}

	async function chooseBackup(): Promise<void> {
		error = null;
		backups = null;
		choosing = true;
		try {
			backups = await listBackups();
		} catch (reason) {
			choosing = false;
			error = String(reason);
		}
	}

	function pick(backup: Backup): void {
		chosen = backup;
		choosing = false;
		confirming = true;
	}

	/** The app restarts with the backup, so this only returns if something went wrong. */
	async function restore(): Promise<void> {
		if (!chosen) return;
		restoring = true;
		try {
			await restoreBackup(chosen.name);
		} catch (reason) {
			confirming = false;
			restoring = false;
			error = String(reason);
		}
	}

	async function run(action: () => Promise<string>): Promise<void> {
		error = null;
		saved = null;
		try {
			saved = await action();
			toast('Saved');
		} catch (reason) {
			error = String(reason);
		}
	}
</script>

{#snippet row(label: string, helper: string)}
	<div class="flex min-w-0 flex-col gap-0.5">
		<span class="text-sm">{label}</span>
		{#if helper}<span class="text-xs text-muted-foreground">{helper}</span>{/if}
	</div>
{/snippet}

<div class="mx-auto flex max-w-180 flex-col gap-7 px-10 pt-9 pb-10">
	<h1 class="font-serif text-3xl font-normal">Settings</h1>

	{#if error}
		<Notice>{error}</Notice>
	{/if}

	<section class="flex flex-col gap-3">
		<h2 class="text-sm font-medium">AI</h2>
		<div class="flex items-center justify-between gap-6 rounded-2xl border bg-card p-5">
			{@render row('Claude Code', 'Cards are created by Claude Opus through your own sign-in.')}
			<span class="shrink-0 text-sm text-muted-foreground">{claudeLabel}</span>
		</div>
	</section>

	{#if settings}
		<section class="flex flex-col gap-3">
			<h2 class="text-sm font-medium">Reviewing</h2>
			<div class="flex flex-col divide-y rounded-2xl border bg-card">
				<div class="flex items-center justify-between gap-6 p-5">
					{@render row('New cards per day', '')}
					<div class="flex items-center gap-0.5 rounded-lg bg-accent/70 p-1">
						<Button
							variant="ghost"
							size="icon"
							aria-label="Fewer new cards"
							onclick={() => stepNewCards(-NEW_CARDS_STEP)}
							class="rounded-md text-foreground-secondary hover:bg-card hover:text-foreground"
						>
							<Minus />
						</Button>
						<Input
							inputmode="numeric"
							value={settings.new_cards_per_day}
							onchange={changeNewCards}
							aria-label="New cards per day"
							class="h-8 w-14 rounded-md border-0 bg-card text-center font-mono shadow-xs dark:bg-card"
						/>
						<Button
							variant="ghost"
							size="icon"
							aria-label="More new cards"
							onclick={() => stepNewCards(NEW_CARDS_STEP)}
							class="rounded-md text-foreground-secondary hover:bg-card hover:text-foreground"
						>
							<Plus />
						</Button>
					</div>
				</div>
			</div>
		</section>

		<section class="flex flex-col gap-3">
			<h2 class="text-sm font-medium">Appearance</h2>
			<div class="flex flex-col divide-y rounded-2xl border bg-card">
				<div class="flex items-center justify-between gap-6 p-5">
					{@render row('Theme', '')}
					<ToggleGroup.Root type="single" value={settings.theme} onValueChange={chooseTheme}>
						<ToggleGroup.Item value="light">Light</ToggleGroup.Item>
						<ToggleGroup.Item value="dark">Dark</ToggleGroup.Item>
						<ToggleGroup.Item value="system">System</ToggleGroup.Item>
					</ToggleGroup.Root>
				</div>
				<div class="flex items-center justify-between gap-6 p-5">
					{@render row('Size', 'Everything grows together, for big screens. ⌘ + and ⌘ − work anywhere.')}
					<ToggleGroup.Root type="single" value={settings.size} onValueChange={chooseSize}>
						<ToggleGroup.Item value="default">Default</ToggleGroup.Item>
						<ToggleGroup.Item value="larger">Larger</ToggleGroup.Item>
						<ToggleGroup.Item value="largest">Largest</ToggleGroup.Item>
					</ToggleGroup.Root>
				</div>
			</div>
		</section>
	{/if}

	<section class="flex flex-col gap-3">
		<h2 class="text-sm font-medium">Data</h2>
		<div class="flex flex-col divide-y rounded-2xl border bg-card">
			<div class="flex items-center justify-between gap-6 p-5">
				{@render row('Backups', `One a day, the last 14 kept, in ${folder}/backups`)}
				<Button variant="secondary" onclick={() => run(backUpNow)} class="shrink-0 border-input">
					Back up now
				</Button>
			</div>
			<div class="flex items-center justify-between gap-6 p-5">
				{@render row('Export', 'All cards as a readable JSON file.')}
				<Button variant="secondary" onclick={() => run(exportCards)} class="shrink-0 border-input">
					Export cards
				</Button>
			</div>
			<div class="flex items-center justify-between gap-6 p-5">
				{@render row('Restore', 'Go back to an earlier backup. Your cards now are backed up first.')}
				<Button variant="secondary" onclick={chooseBackup} class="shrink-0 border-input">
					Restore…
				</Button>
			</div>
		</div>
		{#if saved}
			<p class="text-xs text-muted-foreground">Saved to {saved}</p>
		{/if}
	</section>
</div>

<Dialog.Root bind:open={choosing}>
	<Dialog.Content showCloseButton={false} class="gap-6 p-7 shadow-popover sm:max-w-120">
		<Dialog.Header class="gap-1.5">
			<Dialog.Title class="font-serif text-2xl leading-tight font-normal">
				Restore a backup
			</Dialog.Title>
			<Dialog.Description>
				Myosotis restarts with the cards from the backup you choose.
			</Dialog.Description>
		</Dialog.Header>
		{#if backups === null}
			<div class="flex flex-col gap-2">
				<Skeleton class="h-12 w-full rounded-xl" />
				<Skeleton class="h-12 w-full rounded-xl" />
			</div>
		{:else if backups.length === 0}
			<p class="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
				No backups yet.
			</p>
		{:else}
			<div class="flex max-h-80 flex-col gap-1 overflow-y-auto">
				{#each backups as backup, index (backup.name)}
					<Button
						variant="ghost"
						onclick={() => pick(backup)}
						disabled={backup.card_count === null}
						class="h-auto justify-between gap-6 rounded-xl px-4 py-3 font-normal"
					>
						<span class="flex items-baseline gap-2">
							<span class="text-sm">{day(backup)}</span>
							<span class="font-mono text-xs text-muted-foreground">{time(backup)}</span>
							{#if index === 0}
								<span class="text-xs text-muted-foreground">· latest</span>
							{/if}
						</span>
						<span class="font-mono text-xs text-muted-foreground">{cards(backup)}</span>
					</Button>
				{/each}
			</div>
		{/if}
		<Button variant="ghost" onclick={() => (choosing = false)} class="justify-self-end">
			Cancel
		</Button>
	</Dialog.Content>
</Dialog.Root>

<AlertDialog.Root bind:open={confirming}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>
				{#if chosen}Restore the backup from {day(chosen)}, {time(chosen)}?{/if}
			</AlertDialog.Title>
			<AlertDialog.Description>
				{#if chosen}
					Myosotis will restart with its {cards(chosen)}. What you have now is backed up first, so
					you can come back to it.
				{/if}
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel disabled={restoring}>Keep current</AlertDialog.Cancel>
			<AlertDialog.Action onclick={restore} disabled={restoring}>
				{restoring ? 'Restoring…' : 'Restore and restart'}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
