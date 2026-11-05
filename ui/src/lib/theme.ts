import type { Theme } from './api.js';

let current: Theme = 'system';

function systemIsDark(): boolean {
	return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export function applyTheme(theme: Theme): void {
	current = theme;
	const dark: boolean = theme === 'dark' || (theme === 'system' && systemIsDark());
	document.documentElement.classList.toggle('dark', dark);
}

/** Keeps the app in step with the system while the theme is "system". Returns a cleanup. */
export function watchSystemTheme(): () => void {
	const query: MediaQueryList = window.matchMedia('(prefers-color-scheme: dark)');
	const onChange = (): void => applyTheme(current);
	query.addEventListener('change', onChange);
	return () => query.removeEventListener('change', onChange);
}
