<script lang="ts">
	import { Toaster } from 'svelte-sonner';
	import '../app.css';
	import { getSettings, type Settings } from '#lib/api.js';
	import { applyTheme, watchSystemTheme } from '#lib/theme.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	$effect(() => {
		getSettings().then((settings: Settings) => applyTheme(settings.theme));
		return watchSystemTheme();
	});
</script>

<!-- macOS overlay title bar: the window is dragged from this strip. -->
<div data-tauri-drag-region class="fixed inset-x-0 top-0 z-50 h-7"></div>

{@render children()}

<Toaster position="bottom-center" />
