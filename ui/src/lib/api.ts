import { invoke } from '@tauri-apps/api/core';

export type Grade = 'again' | 'hard' | 'good' | 'easy';
export type Depth = 'essentials' | 'balanced' | 'thorough';
export type ImportStatus = 'generating' | 'in_inbox' | 'failed' | 'done';

export type GradePreview = { grade: Grade; interval: string };

export type SessionCard = {
	id: number;
	question: string;
	answer: string;
	source_title: string | null;
	excerpt: string | null;
	is_new: boolean;
	grades: GradePreview[];
};

export type DraftCard = { id: number; question: string; answer: string; excerpt: string | null };

export type InboxImport = {
	id: number;
	title: string;
	status: ImportStatus;
	error: string | null;
	text: string | null;
	drafts: DraftCard[];
};

export type RecentImport = {
	id: number;
	title: string;
	status: ImportStatus;
	created_at: string;
	card_count: number;
	draft_count: number;
};

export type TodaySummary = {
	due_reviews: number;
	due_new: number;
	minutes: number;
	next_due: string | null;
	inbox_count: number;
	forecast: number[];
	recent_imports: RecentImport[];
};

export type NewImport = { title: string; text: string; depth: Depth; focus: string | null };

export const todaySummary = (): Promise<TodaySummary> => invoke('today_summary');
export const reviewSession = (): Promise<SessionCard[]> => invoke('review_session');
export const gradeCard = (card: number, grade: Grade): Promise<void> =>
	invoke('grade_card', { card, grade });
export const startImport = (source: NewImport): Promise<number> =>
	invoke('start_import', { source });
export const retryImport = (id: number): Promise<void> => invoke('retry_import', { import: id });
export const discardImport = (id: number): Promise<void> =>
	invoke('discard_import', { import: id });
export const inbox = (): Promise<InboxImport[]> => invoke('inbox');
export const acceptDraft = (card: number): Promise<void> => invoke('accept_draft', { card });
export const acceptAllDrafts = (id: number): Promise<void> =>
	invoke('accept_all_drafts', { import: id });
export const editDraft = (card: number, question: string, answer: string): Promise<void> =>
	invoke('edit_draft', { card, question, answer });
export const deleteDraft = (card: number): Promise<void> => invoke('delete_draft', { card });
