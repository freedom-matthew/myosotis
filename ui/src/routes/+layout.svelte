<script lang="ts">
	import { Toaster } from 'svelte-sonner';
	import '../app.css';
	import { getSettings, type Settings } from '#lib/api.js';
	import { stepSize } from '#lib/size.js';
	import { applyTheme, watchSystemTheme } from '#lib/theme.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	$effect(() => {
		getSettings().then((settings: Settings) => applyTheme(settings.theme));
		return watchSystemTheme();
	});

	function onkeydown(event: KeyboardEvent): void {
		if (!event.metaKey && !event.ctrlKey) return;
		const steps: Record<string, 1 | -1 | 0> = { '=': 1, '+': 1, '-': -1, '0': 0 };
		const step: 1 | -1 | 0 | undefined = steps[event.key];
		if (step === undefined) return;
		event.preventDefault();
		stepSize(step);
	}
</script>

<svelte:window {onkeydown} />

<!-- macOS overlay title bar: the window is dragged from this strip. -->
<div data-tauri-drag-region class="fixed inset-x-0 top-0 z-50 h-7"></div>

{@render children()}

<Toaster position="bottom-center" />
