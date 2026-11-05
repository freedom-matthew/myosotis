/** Single-letter shortcuts stay inactive while the user is typing. */
export function isTyping(event: KeyboardEvent): boolean {
	const target: EventTarget | null = event.target;
	return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
}
