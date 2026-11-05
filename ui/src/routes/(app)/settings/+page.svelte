<script lang="ts">
	import { toast } from 'svelte-sonner';
	import {
		backUpNow,
		claudeStatus,
		dataFolder,
		exportCards,
		getSettings,
		updateSettings,
		type ClaudeStatus,
		type Settings,
		type Theme
	} from '#lib/api.js';
	import Notice from '#lib/components/notice.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Slider } from '#lib/components/ui/slider/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import { applyTheme } from '#lib/theme.js';

	let settings: Settings | null = $state<Settings | null>(null);
	let claude: ClaudeStatus | null = $state<ClaudeStatus | null>(null);
	let folder: string = $state('');
	let saved: string | null = $state(null);
	let error: string | null = $state(null);

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

	function changeNewCards(event: Event): void {
		const input: HTMLInputElement = event.currentTarget as HTMLInputElement;
		const count: number = Number.parseInt(input.value, 10);
		if (Number.isInteger(count) && count >= 0) save({ new_cards_per_day: count });
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
					<Input
						type="number"
						min="0"
						value={settings.new_cards_per_day}
						onchange={changeNewCards}
						aria-label="New cards per day"
						class="h-9 w-20 text-right font-mono"
					/>
				</div>
				<div class="flex items-center justify-between gap-6 p-5">
					{@render row('Desired retention', 'Higher means more reviews and better recall.')}
					<div class="flex w-56 items-center gap-3">
						<Slider
							type="single"
							min={80}
							max={97}
							step={1}
							value={settings.desired_retention_percent}
							onValueCommit={(value: number) => save({ desired_retention_percent: value })}
						/>
						<span class="w-10 text-right font-mono text-sm">
							{settings.desired_retention_percent}%
						</span>
					</div>
				</div>
			</div>
		</section>

		<section class="flex flex-col gap-3">
			<h2 class="text-sm font-medium">Appearance</h2>
			<div class="flex items-center justify-between gap-6 rounded-2xl border bg-card p-5">
				{@render row('Theme', '')}
				<ToggleGroup.Root type="single" value={settings.theme} onValueChange={chooseTheme}>
					<ToggleGroup.Item value="light">Light</ToggleGroup.Item>
					<ToggleGroup.Item value="dark">Dark</ToggleGroup.Item>
					<ToggleGroup.Item value="system">System</ToggleGroup.Item>
				</ToggleGroup.Root>
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
		</div>
		{#if saved}
			<p class="text-xs text-muted-foreground">Saved to {saved}</p>
		{/if}
	</section>
</div>
