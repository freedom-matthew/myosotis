import { getSettings, updateSettings, type Settings, type Size } from './api.js';

const SIZES: Size[] = ['default', 'larger', 'largest'];

/** Fired on `window` when the size changes outside Settings, so Settings can follow. */
export const SETTINGS_CHANGED: string = 'myosotis:settings-changed';

/** ⌘+ and ⌘− step through the sizes; ⌘0 goes back to the default. */
export async function stepSize(step: 1 | -1 | 0): Promise<void> {
	const settings: Settings = await getSettings();
	const position: number = SIZES.indexOf(settings.size);
	const next: Size =
		step === 0 ? 'default' : SIZES[Math.min(SIZES.length - 1, Math.max(0, position + step))];
	if (next === settings.size) return;
	const saved: Settings = await updateSettings({ ...settings, size: next });
	window.dispatchEvent(new CustomEvent<Settings>(SETTINGS_CHANGED, { detail: saved }));
}
