<script lang="ts">
	import '../app.css';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	// Follows the system theme until Settings exists.
	$effect(() => {
		const query: MediaQueryList = window.matchMedia('(prefers-color-scheme: dark)');
		const apply = (): void => {
			document.documentElement.classList.toggle('dark', query.matches);
		};
		apply();
		query.addEventListener('change', apply);
		return () => query.removeEventListener('change', apply);
	});
</script>

<!-- macOS overlay title bar: the window is dragged from this strip. -->
<div data-tauri-drag-region class="fixed inset-x-0 top-0 z-50 h-7"></div>

{@render children()}
