import { invoke } from '@tauri-apps/api/core';

export type CardKind = 'qa' | 'cloze' | 'reverse';
export type Grade = 'again' | 'hard' | 'good' | 'easy';
export type Depth = 'essentials' | 'balanced' | 'thorough';
export type ImportStatus = 'generating' | 'in_inbox' | 'failed' | 'done';

export type GradePreview = { grade: Grade; interval: string };

export type SessionCard = {
	id: number;
	kind: CardKind;
	question: string;
	answer: string;
	source_title: string | null;
	excerpt: string | null;
	is_new: boolean;
	grades: GradePreview[];
};

export type ExistingCard = {
	id: number;
	question: string;
	answer: string;
	source_title: string | null;
	review_count: number;
};

export type DraftCard = {
	id: number;
	kind: CardKind;
	question: string;
	answer: string;
	excerpt: string | null;
	duplicate: ExistingCard | null;
};

export type InboxImport = {
	id: number;
	title: string;
	status: ImportStatus;
	error: string | null;
	text: string | null;
	parts_total: number;
	parts_done: number;
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
	retention_percent: number | null;
	slipping_count: number;
	forecast: number[];
	recent_imports: RecentImport[];
};

export type LibraryCard = {
	id: number;
	kind: CardKind;
	question: string;
	answer: string;
	source_title: string | null;
	excerpt: string | null;
	is_new: boolean;
	suspended: boolean;
	review_count: number;
	lapses: number;
	recall_percent: number | null;
	next_due: string;
};

export type Theme = 'system' | 'light' | 'dark';

export type Settings = {
	new_cards_per_day: number;
	desired_retention_percent: number;
	theme: Theme;
};

export type ClaudeStatus = { installed: boolean; logged_in: boolean; plan: string | null };

export type TidyMember = {
	id: number;
	question: string;
	answer: string;
	source_title: string | null;
	review_count: number;
	keeps_history: boolean;
};

export type TidyGroup = {
	members: TidyMember[];
	suggested_question: string;
	suggested_answer: string;
};

export type NewImport = {
	title: string;
	text: string;
	depth: Depth;
	focus: string | null;
	fill_in_blank: boolean;
};

export const todaySummary = (): Promise<TodaySummary> => invoke('today_summary');
export const reviewSession = (): Promise<SessionCard[]> => invoke('review_session');
export const gradeCard = (card: number, grade: Grade): Promise<void> =>
	invoke('grade_card', { card, grade });
export const undoGrade = (card: number): Promise<void> => invoke('undo_grade', { card });
export const editCard = (card: number, question: string, answer: string): Promise<void> =>
	invoke('edit_card', { card, question, answer });
export const explainCard = (card: number): Promise<string> => invoke('explain_card', { card });
export const libraryCards = (): Promise<LibraryCard[]> => invoke('library_cards');
export const setSuspended = (card: number, suspended: boolean): Promise<void> =>
	invoke('set_suspended', { card, suspended });
export const deleteCard = (card: number): Promise<void> => invoke('delete_card', { card });
export const getSettings = (): Promise<Settings> => invoke('get_settings');
export const updateSettings = (settings: Settings): Promise<Settings> =>
	invoke('update_settings', { settings });
export const claudeStatus = (): Promise<ClaudeStatus> => invoke('claude_status');
export const backUpNow = (): Promise<string> => invoke('back_up_now');
export const exportCards = (): Promise<string> => invoke('export_cards');
export const dataFolder = (): Promise<string> => invoke('data_folder');
export const readSourceFile = async (file: File): Promise<string> =>
	invoke('read_source_file', await file.arrayBuffer(), {
		headers: { 'x-file-name': encodeURIComponent(file.name) }
	});
export const tidyGroups = (): Promise<TidyGroup[]> => invoke('tidy_groups');
export const mergeGroup = (
	keeper: number,
	others: number[],
	question: string,
	answer: string
): Promise<void> => invoke('merge_group', { keeper, others, question, answer });
export const markNotDuplicates = (cards: number[]): Promise<void> =>
	invoke('mark_not_duplicates', { cards });
export const stopImport = (id: number): Promise<void> => invoke('stop_import', { import: id });
export const rewriteDraft = (card: number): Promise<void> => invoke('rewrite_draft', { card });
export const keepBothCards = (card: number): Promise<void> => invoke('keep_both_cards', { card });
export const mergeSourceIntoExisting = (card: number): Promise<void> =>
	invoke('merge_source_into_existing', { card });
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
