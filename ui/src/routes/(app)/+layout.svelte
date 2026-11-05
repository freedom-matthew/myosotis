<script lang="ts">
	import { afterNavigate, goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Inbox, Library, Plus, Sun } from '@lucide/svelte';
	import { inbox, type InboxImport } from '#lib/api.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { cn } from '#lib/utils.js';
	import type { LayoutProps } from './$types';

	const REFRESH_MS: number = 3000;

	let { children }: LayoutProps = $props();
	let inboxCount: number = $state(0);

	async function refreshCount(): Promise<void> {
		const imports: InboxImport[] = await inbox();
		inboxCount = imports.reduce((total, item) => total + item.drafts.length, 0);
	}

	afterNavigate(() => {
		refreshCount();
	});

	$effect(() => {
		const timer: ReturnType<typeof setInterval> = setInterval(refreshCount, REFRESH_MS);
		return () => clearInterval(timer);
	});

	function onkeydown(event: KeyboardEvent): void {
		if (!event.metaKey && !event.ctrlKey) return;
		const routes: Record<string, string> = {
			n: '/new',
			'1': '/',
			'2': '/inbox',
			'3': '/library'
		};
		const route: string | undefined = routes[event.key];
		if (route) {
			event.preventDefault();
			goto(route);
		}
	}

	const navItem = (href: string): string =>
		cn(
			'flex items-center gap-2.5 rounded-md px-3 py-2 text-sm transition-colors hover:bg-sidebar-accent',
			page.url.pathname === href
				? 'bg-sidebar-accent font-medium text-sidebar-accent-foreground'
				: 'text-sidebar-foreground'
		);
</script>

<svelte:window {onkeydown} />

<div class="flex h-screen overflow-hidden">
	<nav
		aria-label="Main"
		class="flex w-54 shrink-0 flex-col gap-5 border-r bg-sidebar px-3.5 pt-10 pb-5"
	>
		<a href="/" class="px-2.5 font-serif text-2xl leading-none italic">myosotis</a>
		<a
			href="/new"
			class="flex items-center justify-between rounded-lg bg-sidebar-primary px-3 py-2 text-sm font-medium text-sidebar-primary-foreground"
		>
			<span class="flex items-center gap-2"><Plus size={16} />New source</span>
			<span class="font-mono text-xs opacity-70">⌘N</span>
		</a>
		<div class="flex flex-col gap-0.5">
			<a href="/" class={navItem('/')}><Sun size={16} strokeWidth={1.8} />Today</a>
			<a href="/inbox" class={navItem('/inbox')}>
				<Inbox size={16} strokeWidth={1.8} />
				<span class="flex-1">Inbox</span>
				{#if inboxCount > 0}
					<Badge class="bg-primary-tint font-mono text-primary-strong">{inboxCount}</Badge>
				{/if}
			</a>
			<a href="/library" class={navItem('/library')}>
				<Library size={16} strokeWidth={1.8} />Library
			</a>
		</div>
	</nav>

	<main class="min-w-0 flex-1 overflow-y-auto">
		{@render children()}
	</main>
</div>
