use std::error::Error;
use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use myosotis_core::{
    Card, CardId, CardKind, CardRepo, Depth, DraftCard, DraftToStore, DueCard, ExistingCard,
    ExplanationRequest, GeneratedCard, GenerationRequest, Grade, ImportId, ImportRepo,
    ImportStatus, InboxImport, LibraryRepo, NewImport, Phase, RecentImport, RepoError, ReviewRepo,
    Schedule, Settings, SettingsRepo, StoredCard, Theme, TidyRepo,
};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteTransaction};

/// `Clone` is cheap: the pool is shared, like a C# `HttpClient` passed around.
#[derive(Clone)]
pub struct SqliteCardRepo {
    pool: SqlitePool,
}

impl SqliteCardRepo {
    /// Opens (or creates) the database file and brings its schema up to date.
    pub async fn open(path: &Path) -> Result<Self, RepoError> {
        let options: SqliteConnectOptions = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool: SqlitePool = SqlitePool::connect_with(options)
            .await
            .map_err(repo_error)?;
        let repo: Self = Self::migrated(pool).await?;
        repo.fail_interrupted_imports().await?;
        Ok(repo)
    }

    /// Fresh, empty database that lives only as long as the repo. Meant for tests.
    pub async fn in_memory() -> Result<Self, RepoError> {
        // Every connection to `:memory:` is its own database, so keep exactly one, forever.
        let pool: SqlitePool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect("sqlite::memory:")
            .await
            .map_err(repo_error)?;
        Self::migrated(pool).await
    }

    async fn migrated(pool: SqlitePool) -> Result<Self, RepoError> {
        sqlx::migrate!().run(&pool).await.map_err(repo_error)?;
        Ok(Self { pool })
    }

    /// Imports still "generating" at startup died with the previous run.
    async fn fail_interrupted_imports(&self) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE import SET status = 'failed', error = 'The app closed before this finished.'
             WHERE status = 'generating'"
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    /// Adds an active card. New cards are due from the epoch: ready whenever a session starts.
    pub async fn insert(&self, card: &Card) -> Result<(), RepoError> {
        let created_at: String = to_db(Utc::now());
        sqlx::query!(
            "INSERT INTO card (question, answer, created_at) VALUES (?, ?, ?)",
            card.question,
            card.answer,
            created_at
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    pub async fn review_count(&self, card: CardId) -> Result<i64, RepoError> {
        sqlx::query_scalar!("SELECT COUNT(*) FROM review WHERE card_id = ?", card)
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)
    }

    pub async fn source_text(&self, import: ImportId) -> Result<Option<String>, RepoError> {
        sqlx::query_scalar!("SELECT full_text FROM import WHERE id = ?", import)
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)
    }

    /// Writes a complete, consistent copy of the database, safe while the app is running.
    pub async fn back_up_to(&self, path: &Path) -> Result<(), RepoError> {
        sqlx::query("VACUUM INTO ?")
            .bind(path.to_string_lossy().into_owned())
            .execute(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(())
    }

    async fn begin(&self) -> Result<SqliteTransaction<'static>, RepoError> {
        self.pool.begin().await.map_err(repo_error)
    }
}

impl CardRepo for SqliteCardRepo {
    async fn list(&self) -> Result<Vec<Card>, RepoError> {
        sqlx::query_as!(
            Card,
            "SELECT question, answer FROM card WHERE status = 'active' ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)
    }
}

impl ReviewRepo for SqliteCardRepo {
    async fn due_cards(&self, until: DateTime<Utc>) -> Result<Vec<DueCard>, RepoError> {
        let until: String = to_db(until);
        let rows = sqlx::query!(
            r#"SELECT c.id AS "id!", c.kind, c.sibling_id, c.question, c.answer, c.due, c.stability,
                      c.difficulty,
                      c.elapsed_days, c.scheduled_days, c.reps, c.lapses, c.phase, c.last_review,
                      o.source_title AS "source_title?", o.excerpt AS "excerpt?"
               FROM card c
               LEFT JOIN origin o
                 ON o.id = (SELECT MIN(id) FROM origin WHERE card_id = c.id)
               WHERE c.status = 'active' AND c.suspended = 0 AND c.due <= ?
               ORDER BY c.due, c.id"#,
            until
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(DueCard {
                    id: row.id,
                    kind: CardKind::parse(&row.kind),
                    sibling: row.sibling_id,
                    card: Card::new(row.question, row.answer),
                    source_title: row.source_title,
                    excerpt: row.excerpt,
                    schedule: Schedule {
                        due: from_db(&row.due)?,
                        stability: row.stability,
                        difficulty: row.difficulty,
                        elapsed_days: row.elapsed_days,
                        scheduled_days: row.scheduled_days,
                        reps: row.reps as i32,
                        lapses: row.lapses as i32,
                        phase: Phase::parse(&row.phase),
                        last_review: from_db(&row.last_review)?,
                    },
                })
            })
            .collect()
    }

    async fn new_cards_started_since(&self, since: DateTime<Utc>) -> Result<usize, RepoError> {
        let since: String = to_db(since);
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM review WHERE was_new = 1 AND reviewed_at >= ?",
            since
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(count as usize)
    }

    async fn cards_reviewed_since(&self, since: DateTime<Utc>) -> Result<Vec<CardId>, RepoError> {
        let since: String = to_db(since);
        sqlx::query_scalar!(
            "SELECT DISTINCT card_id FROM review WHERE reviewed_at >= ?",
            since
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn retention_since(&self, since: DateTime<Utc>) -> Result<Option<u8>, RepoError> {
        let since: String = to_db(since);
        let row = sqlx::query!(
            r#"SELECT COUNT(*) AS "total!: i64",
                      COUNT(CASE WHEN grade != 'again' THEN 1 END) AS "remembered!: i64"
               FROM review WHERE was_new = 0 AND reviewed_at >= ?"#,
            since
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok((row.total > 0).then(|| (row.remembered * 100 / row.total) as u8))
    }

    async fn slipping_count(&self) -> Result<i64, RepoError> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!: i64" FROM card
               WHERE status = 'active' AND suspended = 0 AND lapses >= 3"#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn schedule_of(&self, card: CardId) -> Result<Schedule, RepoError> {
        let row = sqlx::query!(
            "SELECT due, stability, difficulty, elapsed_days, scheduled_days, reps, lapses,
                    phase, last_review
             FROM card WHERE id = ?",
            card
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)?;

        Ok(Schedule {
            due: from_db(&row.due)?,
            stability: row.stability,
            difficulty: row.difficulty,
            elapsed_days: row.elapsed_days,
            scheduled_days: row.scheduled_days,
            reps: row.reps as i32,
            lapses: row.lapses as i32,
            phase: Phase::parse(&row.phase),
            last_review: from_db(&row.last_review)?,
        })
    }

    async fn record_review(
        &self,
        card: CardId,
        grade: Grade,
        before: &Schedule,
        after: &Schedule,
        now: DateTime<Utc>,
    ) -> Result<(), RepoError> {
        let reviewed_at: String = to_db(now);
        let next_due: String = to_db(after.due);
        let grade: &str = grade.as_str();
        let was_new: bool = before.is_new();
        let previous_due: String = to_db(before.due);
        let previous_phase: &str = before.phase.as_str();
        let previous_last_review: String = to_db(before.last_review);
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        sqlx::query!(
            "INSERT INTO review (
                 card_id, grade, was_new, reviewed_at, next_due,
                 previous_due, previous_stability, previous_difficulty,
                 previous_elapsed_days, previous_scheduled_days, previous_reps,
                 previous_lapses, previous_phase, previous_last_review
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            card,
            grade,
            was_new,
            reviewed_at,
            next_due,
            previous_due,
            before.stability,
            before.difficulty,
            before.elapsed_days,
            before.scheduled_days,
            before.reps,
            before.lapses,
            previous_phase,
            previous_last_review
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?;
        save_schedule(&mut transaction, card, after).await?;
        transaction.commit().await.map_err(repo_error)
    }

    async fn undo_review(&self, card: CardId) -> Result<(), RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        let row = sqlx::query!(
            r#"SELECT id AS "id!", previous_due, previous_stability, previous_difficulty,
                      previous_elapsed_days, previous_scheduled_days, previous_reps,
                      previous_lapses, previous_phase, previous_last_review
               FROM review
               WHERE card_id = ?
               ORDER BY id DESC
               LIMIT 1"#,
            card
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(repo_error)?;

        // Reviews from before undo existed have no snapshot, so they stay as they are.
        if let (Some(due), Some(phase), Some(last_review)) = (
            row.previous_due,
            row.previous_phase,
            row.previous_last_review,
        ) {
            let schedule: Schedule = Schedule {
                due: from_db(&due)?,
                stability: row.previous_stability.unwrap_or_default(),
                difficulty: row.previous_difficulty.unwrap_or_default(),
                elapsed_days: row.previous_elapsed_days.unwrap_or_default(),
                scheduled_days: row.previous_scheduled_days.unwrap_or_default(),
                reps: row.previous_reps.unwrap_or_default() as i32,
                lapses: row.previous_lapses.unwrap_or_default() as i32,
                phase: Phase::parse(&phase),
                last_review: from_db(&last_review)?,
            };
            save_schedule(&mut transaction, card, &schedule).await?;
            sqlx::query!("DELETE FROM review WHERE id = ?", row.id)
                .execute(&mut *transaction)
                .await
                .map_err(repo_error)?;
        }
        transaction.commit().await.map_err(repo_error)
    }

    async fn explanation_request(&self, card: CardId) -> Result<ExplanationRequest, RepoError> {
        sqlx::query_as!(
            ExplanationRequest,
            r#"SELECT c.question, c.answer, o.excerpt AS "excerpt?"
               FROM card c
               LEFT JOIN origin o
                 ON o.id = (SELECT MIN(id) FROM origin WHERE card_id = c.id)
               WHERE c.id = ?"#,
            card
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn update_card(&self, card: CardId, content: &Card) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE card SET question = ?, answer = ? WHERE id = ? AND status = 'active'",
            content.question,
            content.answer,
            card
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn all_due_dates(&self) -> Result<Vec<DateTime<Utc>>, RepoError> {
        let rows: Vec<String> =
            sqlx::query_scalar!("SELECT due FROM card WHERE status = 'active' AND suspended = 0")
                .fetch_all(&self.pool)
                .await
                .map_err(repo_error)?;
        rows.iter().map(|due| from_db(due)).collect()
    }
}

async fn save_schedule(
    transaction: &mut SqliteTransaction<'static>,
    card: CardId,
    schedule: &Schedule,
) -> Result<(), RepoError> {
    let due: String = to_db(schedule.due);
    let last_review: String = to_db(schedule.last_review);
    let phase: &str = schedule.phase.as_str();
    sqlx::query!(
        "UPDATE card
         SET due = ?, stability = ?, difficulty = ?, elapsed_days = ?, scheduled_days = ?,
             reps = ?, lapses = ?, phase = ?, last_review = ?
         WHERE id = ?",
        due,
        schedule.stability,
        schedule.difficulty,
        schedule.elapsed_days,
        schedule.scheduled_days,
        schedule.reps,
        schedule.lapses,
        phase,
        last_review,
        card
    )
    .execute(&mut **transaction)
    .await
    .map_err(repo_error)?;
    Ok(())
}

impl SettingsRepo for SqliteCardRepo {
    async fn load_settings(&self) -> Result<Settings, RepoError> {
        let rows = sqlx::query!(r#"SELECT key AS "key!", value FROM setting"#)
            .fetch_all(&self.pool)
            .await
            .map_err(repo_error)?;

        let mut settings: Settings = Settings::default();
        for row in rows {
            match row.key.as_str() {
                "new_cards_per_day" => {
                    settings.new_cards_per_day =
                        row.value.parse().unwrap_or(settings.new_cards_per_day);
                }
                "desired_retention_percent" => {
                    settings.desired_retention_percent = row
                        .value
                        .parse()
                        .unwrap_or(settings.desired_retention_percent);
                }
                "theme" => settings.theme = Theme::parse(&row.value),
                _ => {}
            }
        }
        Ok(settings)
    }

    async fn save_settings(&self, settings: &Settings) -> Result<(), RepoError> {
        let values: [(&str, String); 3] = [
            ("new_cards_per_day", settings.new_cards_per_day.to_string()),
            (
                "desired_retention_percent",
                settings.desired_retention_percent.to_string(),
            ),
            ("theme", settings.theme.as_str().to_string()),
        ];
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        for (key, value) in values {
            sqlx::query!(
                "INSERT INTO setting (key, value) VALUES (?, ?)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                key,
                value
            )
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        }
        transaction.commit().await.map_err(repo_error)
    }
}

impl TidyRepo for SqliteCardRepo {
    async fn not_duplicate_pairs(&self) -> Result<Vec<(CardId, CardId)>, RepoError> {
        let rows = sqlx::query!("SELECT card_a, card_b FROM not_duplicate")
            .fetch_all(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(rows
            .into_iter()
            .map(|row| (row.card_a, row.card_b))
            .collect())
    }

    async fn remember_not_duplicates(&self, cards: &[CardId]) -> Result<(), RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        for (index, first) in cards.iter().enumerate() {
            for second in &cards[index + 1..] {
                sqlx::query!(
                    "INSERT OR IGNORE INTO not_duplicate (card_a, card_b) VALUES (?, ?)",
                    first,
                    second
                )
                .execute(&mut *transaction)
                .await
                .map_err(repo_error)?;
            }
        }
        transaction.commit().await.map_err(repo_error)
    }

    async fn merge_cards(
        &self,
        keeper: CardId,
        others: &[CardId],
        wording: &Card,
    ) -> Result<(), RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        sqlx::query!(
            "UPDATE card SET question = ?, answer = ? WHERE id = ?",
            wording.question,
            wording.answer,
            keeper
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?;
        for other in others {
            sqlx::query!(
                "UPDATE origin SET card_id = ? WHERE card_id = ?",
                keeper,
                other
            )
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
            sqlx::query!("DELETE FROM review WHERE card_id = ?", other)
                .execute(&mut *transaction)
                .await
                .map_err(repo_error)?;
            sqlx::query!("DELETE FROM card WHERE id = ?", other)
                .execute(&mut *transaction)
                .await
                .map_err(repo_error)?;
        }
        transaction.commit().await.map_err(repo_error)
    }
}

impl LibraryRepo for SqliteCardRepo {
    async fn all_cards(&self) -> Result<Vec<StoredCard>, RepoError> {
        let rows = sqlx::query!(
            r#"SELECT c.id AS "id!", c.kind, c.sibling_id, c.question, c.answer, c.due, c.stability,
                      c.difficulty,
                      c.elapsed_days, c.scheduled_days, c.reps, c.lapses, c.phase, c.last_review,
                      c.suspended AS "suspended: bool",
                      o.source_title AS "source_title?", o.excerpt AS "excerpt?",
                      (SELECT COUNT(*) FROM review r WHERE r.card_id = c.id)
                          AS "review_count!: i64"
               FROM card c
               LEFT JOIN origin o
                 ON o.id = (SELECT MIN(id) FROM origin WHERE card_id = c.id)
               WHERE c.status = 'active'
               ORDER BY c.id DESC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(StoredCard {
                    due: DueCard {
                        id: row.id,
                        kind: CardKind::parse(&row.kind),
                        sibling: row.sibling_id,
                        card: Card::new(row.question, row.answer),
                        source_title: row.source_title,
                        excerpt: row.excerpt,
                        schedule: Schedule {
                            due: from_db(&row.due)?,
                            stability: row.stability,
                            difficulty: row.difficulty,
                            elapsed_days: row.elapsed_days,
                            scheduled_days: row.scheduled_days,
                            reps: row.reps as i32,
                            lapses: row.lapses as i32,
                            phase: Phase::parse(&row.phase),
                            last_review: from_db(&row.last_review)?,
                        },
                    },
                    suspended: row.suspended,
                    review_count: row.review_count,
                })
            })
            .collect()
    }

    async fn set_suspended(&self, card: CardId, suspended: bool) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE card SET suspended = ? WHERE id = ? AND status = 'active'",
            suspended,
            card
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn delete_card(&self, card: CardId) -> Result<(), RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        sqlx::query!("DELETE FROM review WHERE card_id = ?", card)
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        sqlx::query!("DELETE FROM card WHERE id = ? AND status = 'active'", card)
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        transaction.commit().await.map_err(repo_error)
    }
}

impl ImportRepo for SqliteCardRepo {
    async fn create_import(
        &self,
        import: &NewImport,
        now: DateTime<Utc>,
    ) -> Result<ImportId, RepoError> {
        let created_at: String = to_db(now);
        let depth: &str = import.depth.as_str();
        let status: &str = ImportStatus::Generating.as_str();
        sqlx::query_scalar!(
            r#"INSERT INTO import (title, full_text, status, depth, focus, fill_in_blank, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               RETURNING id AS "id!""#,
            import.title,
            import.text,
            status,
            depth,
            import.focus,
            import.fill_in_blank,
            created_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn generation_request(&self, import: ImportId) -> Result<GenerationRequest, RepoError> {
        let row = sqlx::query!(
            r#"SELECT title, full_text, depth, focus, fill_in_blank AS "fill_in_blank: bool"
               FROM import WHERE id = ?"#,
            import
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)?;

        Ok(GenerationRequest {
            title: row.title,
            text: row.full_text.unwrap_or_default(),
            target_cards: Depth::parse(&row.depth).target_cards(),
            focus: row.focus,
            fill_in_blank: row.fill_in_blank,
        })
    }

    async fn mark_generating(&self, import: ImportId, parts_total: usize) -> Result<(), RepoError> {
        let parts_total: i64 = parts_total as i64;
        sqlx::query!(
            "UPDATE import SET status = 'generating', error = NULL, parts_total = ? WHERE id = ?",
            parts_total,
            import
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn parts_done(&self, import: ImportId) -> Result<usize, RepoError> {
        let done: i64 = sqlx::query_scalar!("SELECT parts_done FROM import WHERE id = ?", import)
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(done as usize)
    }

    async fn is_generating(&self, import: ImportId) -> Result<bool, RepoError> {
        let status: String = sqlx::query_scalar!("SELECT status FROM import WHERE id = ?", import)
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(ImportStatus::parse(&status) == ImportStatus::Generating)
    }

    async fn draft_count(&self, import: ImportId) -> Result<i64, RepoError> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!: i64" FROM origin WHERE import_id = ?"#,
            import
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn mark_ready(&self, import: ImportId) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE import SET status = 'in_inbox', error = NULL
             WHERE id = ? AND status = 'generating'",
            import
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn store_drafts(
        &self,
        import: ImportId,
        cards: &[DraftToStore],
    ) -> Result<(), RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        for draft in cards {
            let card: &GeneratedCard = &draft.card;
            let kind: &str = card.kind.as_str();
            let id: i64 = sqlx::query_scalar!(
                r#"INSERT INTO card (kind, question, answer, status, duplicate_of, created_at)
                   VALUES (?, ?, ?, 'draft', ?, (SELECT created_at FROM import WHERE id = ?))
                   RETURNING id AS "id!""#,
                kind,
                card.question,
                card.answer,
                draft.duplicate_of,
                import
            )
            .fetch_one(&mut *transaction)
            .await
            .map_err(repo_error)?;

            sqlx::query!(
                "INSERT INTO origin (card_id, import_id, source_title, excerpt, imported_at)
                 SELECT ?, id, title, ?, created_at FROM import WHERE id = ?",
                id,
                card.excerpt,
                import
            )
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        }
        sqlx::query!(
            "UPDATE import SET parts_done = parts_done + 1 WHERE id = ?",
            import
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?;
        transaction.commit().await.map_err(repo_error)
    }

    async fn mark_generating_finished(&self, import: ImportId) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE import SET status = 'in_inbox', error = NULL
             WHERE id = ? AND status = 'failed'",
            import
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn mark_failed(&self, import: ImportId, message: &str) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE import SET status = 'failed', error = ? WHERE id = ?",
            message,
            import
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn inbox(&self) -> Result<Vec<InboxImport>, RepoError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!", title, status, error, full_text, parts_total, parts_done
               FROM import
               WHERE status IN ('generating', 'in_inbox', 'failed')
               ORDER BY id"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)?;

        let mut imports: Vec<InboxImport> = Vec::with_capacity(rows.len());
        for row in rows {
            let drafts: Vec<DraftCard> = sqlx::query!(
                r#"SELECT c.id AS "id!", c.kind, c.question, c.answer, o.excerpt AS "excerpt?",
                          e.id AS "existing_id?", e.question AS "existing_question?",
                          e.answer AS "existing_answer?",
                          (SELECT source_title FROM origin WHERE card_id = e.id LIMIT 1)
                              AS "existing_source?: String",
                          (SELECT COUNT(*) FROM review WHERE card_id = e.id)
                              AS "existing_reviews!: i64"
                   FROM card c
                   JOIN origin o ON o.card_id = c.id
                   LEFT JOIN card e ON e.id = c.duplicate_of
                   WHERE c.status = 'draft' AND o.import_id = ?
                   ORDER BY c.id"#,
                row.id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(repo_error)?
            .into_iter()
            .map(|draft| DraftCard {
                id: draft.id,
                kind: CardKind::parse(&draft.kind),
                question: draft.question,
                answer: draft.answer,
                excerpt: draft.excerpt,
                duplicate: draft.existing_id.map(|id| ExistingCard {
                    id,
                    question: draft.existing_question.unwrap_or_default(),
                    answer: draft.existing_answer.unwrap_or_default(),
                    source_title: draft.existing_source,
                    review_count: draft.existing_reviews,
                }),
            })
            .collect();

            imports.push(InboxImport {
                id: row.id,
                title: row.title,
                status: ImportStatus::parse(&row.status),
                error: row.error,
                text: row.full_text,
                parts_total: row.parts_total,
                parts_done: row.parts_done,
                drafts,
            });
        }
        Ok(imports)
    }

    async fn draft(&self, card: CardId) -> Result<DraftCard, RepoError> {
        let row = sqlx::query!(
            r#"SELECT c.id AS "id!", c.kind, c.question, c.answer, o.excerpt AS "excerpt?"
               FROM card c
               LEFT JOIN origin o ON o.card_id = c.id
               WHERE c.id = ?"#,
            card
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(DraftCard {
            id: row.id,
            kind: CardKind::parse(&row.kind),
            question: row.question,
            answer: row.answer,
            excerpt: row.excerpt,
            duplicate: None,
        })
    }

    async fn known_card_texts(&self) -> Result<Vec<(CardId, String)>, RepoError> {
        let rows = sqlx::query!(r#"SELECT id AS "id!", question, answer FROM card"#)
            .fetch_all(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(rows
            .into_iter()
            .map(|row| (row.id, format!("{} {}", row.question, row.answer)))
            .collect())
    }

    async fn clear_duplicate(&self, card: CardId) -> Result<(), RepoError> {
        sqlx::query!("UPDATE card SET duplicate_of = NULL WHERE id = ?", card)
            .execute(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(())
    }

    async fn add_source_to_existing(&self, card: CardId) -> Result<ImportId, RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        let import: ImportId =
            sqlx::query_scalar!("SELECT import_id FROM origin WHERE card_id = ?", card)
                .fetch_one(&mut *transaction)
                .await
                .map_err(repo_error)?;
        sqlx::query!(
            "UPDATE origin SET card_id = (SELECT duplicate_of FROM card WHERE id = ?)
             WHERE card_id = ?",
            card,
            card
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?;
        sqlx::query!("DELETE FROM card WHERE id = ? AND status = 'draft'", card)
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        transaction.commit().await.map_err(repo_error)?;
        Ok(import)
    }

    async fn drafts_of(&self, import: ImportId) -> Result<Vec<CardId>, RepoError> {
        sqlx::query_scalar!(
            r#"SELECT c.id AS "id!" FROM card c
               JOIN origin o ON o.card_id = c.id
               WHERE c.status = 'draft' AND o.import_id = ?
               ORDER BY c.id"#,
            import
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn accept_draft(&self, card: CardId, schedule: &Schedule) -> Result<ImportId, RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        let accepted: u64 = sqlx::query!(
            "UPDATE card SET status = 'active' WHERE id = ? AND status = 'draft'",
            card
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?
        .rows_affected();
        // Already accepted: the card may have review progress that must not be reset.
        if accepted > 0 {
            save_schedule(&mut transaction, card, schedule).await?;
            add_reverse_sibling(&mut transaction, card).await?;
        }
        let import: ImportId =
            sqlx::query_scalar!("SELECT import_id FROM origin WHERE card_id = ?", card)
                .fetch_one(&mut *transaction)
                .await
                .map_err(repo_error)?;
        transaction.commit().await.map_err(repo_error)?;
        Ok(import)
    }

    async fn update_draft(&self, card: CardId, content: &Card) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE card SET question = ?, answer = ? WHERE id = ? AND status = 'draft'",
            content.question,
            content.answer,
            card
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn delete_draft(&self, card: CardId) -> Result<ImportId, RepoError> {
        let import: ImportId =
            sqlx::query_scalar!("SELECT import_id FROM origin WHERE card_id = ?", card)
                .fetch_one(&self.pool)
                .await
                .map_err(repo_error)?;
        sqlx::query!("DELETE FROM card WHERE id = ? AND status = 'draft'", card)
            .execute(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(import)
    }

    async fn finish_import_if_empty(&self, import: ImportId) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE import SET status = 'done', full_text = NULL
             WHERE id = ? AND status = 'in_inbox' AND NOT EXISTS (
                 SELECT 1 FROM origin o JOIN card c ON c.id = o.card_id
                 WHERE o.import_id = import.id AND c.status = 'draft'
             )",
            import
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn discard_failed_import(&self, import: ImportId) -> Result<(), RepoError> {
        sqlx::query!(
            "DELETE FROM import WHERE id = ? AND status = 'failed'",
            import
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }

    async fn recent_imports(&self, limit: i64) -> Result<Vec<RecentImport>, RepoError> {
        let rows = sqlx::query!(
            r#"SELECT i.id AS "id!", i.title, i.status, i.created_at,
                      COUNT(CASE WHEN c.status = 'active' THEN 1 END) AS "card_count!: i64",
                      COUNT(CASE WHEN c.status = 'draft' THEN 1 END) AS "draft_count!: i64"
               FROM import i
               LEFT JOIN origin o ON o.import_id = i.id
               LEFT JOIN card c ON c.id = o.card_id
               GROUP BY i.id
               ORDER BY i.id DESC
               LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)?;

        rows.into_iter()
            .map(|row| {
                Ok(RecentImport {
                    id: row.id,
                    title: row.title,
                    status: ImportStatus::parse(&row.status),
                    created_at: from_db(&row.created_at)?,
                    card_count: row.card_count,
                    draft_count: row.draft_count,
                })
            })
            .collect()
    }

    async fn inbox_count(&self) -> Result<i64, RepoError> {
        sqlx::query_scalar!("SELECT COUNT(*) FROM card WHERE status = 'draft'")
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)
    }
}

/// A reverse card is reviewed both ways, so accepting one adds the other direction.
async fn add_reverse_sibling(
    transaction: &mut SqliteTransaction<'static>,
    card: CardId,
) -> Result<(), RepoError> {
    let sibling: Option<i64> = sqlx::query_scalar!(
        r#"INSERT INTO card (kind, question, answer, status, created_at, due, last_review, sibling_id)
           SELECT kind, answer, question, 'active', created_at, due, last_review, id
           FROM card WHERE id = ? AND kind = 'reverse'
           RETURNING id AS "id!""#,
        card
    )
    .fetch_optional(&mut **transaction)
    .await
    .map_err(repo_error)?;

    if let Some(sibling) = sibling {
        sqlx::query!("UPDATE card SET sibling_id = ? WHERE id = ?", sibling, card)
            .execute(&mut **transaction)
            .await
            .map_err(repo_error)?;
        sqlx::query!(
            "INSERT INTO origin (card_id, import_id, source_title, source_kind, excerpt, imported_at)
             SELECT ?, import_id, source_title, source_kind, excerpt, imported_at
             FROM origin WHERE card_id = ?",
            sibling,
            card
        )
        .execute(&mut **transaction)
        .await
        .map_err(repo_error)?;
    }
    Ok(())
}

/// Fixed-width UTC text, so comparing strings in SQL compares times.
fn to_db(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn from_db(value: &str) -> Result<DateTime<Utc>, RepoError> {
    DateTime::from_str(value).map_err(repo_error)
}

fn repo_error(error: impl Error + Send + Sync + 'static) -> RepoError {
    RepoError(Box::new(error))
}
