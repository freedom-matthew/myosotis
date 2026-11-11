import type { CardKind } from '#lib/api.js';

export const KIND_LABELS: Record<CardKind, string> = {
	question: 'Question',
	concept: 'Concept',
	describe: 'Describe'
};
