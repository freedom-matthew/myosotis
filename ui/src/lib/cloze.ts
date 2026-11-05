const GAP: RegExp = /\{\{(.*?)\}\}/;

/** A fill-the-gap question with its gap written out, for lists and search. */
export function withoutGapMarks(question: string): string {
	return question.replace(GAP, '$1');
}

export function splitGap(question: string): { before: string; gap: string; after: string } | null {
	const match: RegExpExecArray | null = GAP.exec(question);
	if (!match) return null;
	return {
		before: question.slice(0, match.index),
		gap: match[1],
		after: question.slice(match.index + match[0].length)
	};
}
