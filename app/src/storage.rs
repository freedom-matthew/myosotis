use std::error::Error;
use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use myosotis_core::{
    Card, CardId, CardRepo, Depth, DraftCard, DueCard, ExplanationRequest, GeneratedCard,
    GenerationRequest, Grade, ImportId, ImportRepo, ImportStatus, InboxImport, LibraryRepo,
    NewImport, Phase, RecentImport, RepoError, ReviewRepo, Schedule, StoredCard,
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
            "UPDATE imports SET status = 'failed', error = 'The app closed before this finished.'
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
            "INSERT INTO cards (question, answer, created_at) VALUES (?, ?, ?)",
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
        sqlx::query_scalar!("SELECT COUNT(*) FROM reviews WHERE card_id = ?", card)
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)
    }

    pub async fn source_text(&self, import: ImportId) -> Result<Option<String>, RepoError> {
        sqlx::query_scalar!("SELECT full_text FROM imports WHERE id = ?", import)
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)
    }

    async fn begin(&self) -> Result<SqliteTransaction<'static>, RepoError> {
        self.pool.begin().await.map_err(repo_error)
    }
}

impl CardRepo for SqliteCardRepo {
    async fn list(&self) -> Result<Vec<Card>, RepoError> {
        sqlx::query_as!(
            Card,
            "SELECT question, answer FROM cards WHERE status = 'active' ORDER BY id"
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
            r#"SELECT c.id AS "id!", c.question, c.answer, c.due, c.stability, c.difficulty,
                      c.elapsed_days, c.scheduled_days, c.reps, c.lapses, c.phase, c.last_review,
                      o.source_title AS "source_title?", o.excerpt AS "excerpt?"
               FROM cards c
               LEFT JOIN origins o
                 ON o.id = (SELECT MIN(id) FROM origins WHERE card_id = c.id)
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
            "SELECT COUNT(*) FROM reviews WHERE was_new = 1 AND reviewed_at >= ?",
            since
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(count as usize)
    }

    async fn schedule_of(&self, card: CardId) -> Result<Schedule, RepoError> {
        let row = sqlx::query!(
            "SELECT due, stability, difficulty, elapsed_days, scheduled_days, reps, lapses,
                    phase, last_review
             FROM cards WHERE id = ?",
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
            "INSERT INTO reviews (
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
               FROM reviews
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
            sqlx::query!("DELETE FROM reviews WHERE id = ?", row.id)
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
               FROM cards c
               LEFT JOIN origins o
                 ON o.id = (SELECT MIN(id) FROM origins WHERE card_id = c.id)
               WHERE c.id = ?"#,
            card
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn update_card(&self, card: CardId, content: &Card) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE cards SET question = ?, answer = ? WHERE id = ? AND status = 'active'",
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
            sqlx::query_scalar!("SELECT due FROM cards WHERE status = 'active' AND suspended = 0")
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
        "UPDATE cards
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

impl LibraryRepo for SqliteCardRepo {
    async fn all_cards(&self) -> Result<Vec<StoredCard>, RepoError> {
        let rows = sqlx::query!(
            r#"SELECT c.id AS "id!", c.question, c.answer, c.due, c.stability, c.difficulty,
                      c.elapsed_days, c.scheduled_days, c.reps, c.lapses, c.phase, c.last_review,
                      c.suspended AS "suspended: bool",
                      o.source_title AS "source_title?", o.excerpt AS "excerpt?",
                      (SELECT COUNT(*) FROM reviews r WHERE r.card_id = c.id)
                          AS "review_count!: i64"
               FROM cards c
               LEFT JOIN origins o
                 ON o.id = (SELECT MIN(id) FROM origins WHERE card_id = c.id)
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
            "UPDATE cards SET suspended = ? WHERE id = ? AND status = 'active'",
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
        sqlx::query!("DELETE FROM reviews WHERE card_id = ?", card)
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        sqlx::query!("DELETE FROM cards WHERE id = ? AND status = 'active'", card)
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
            r#"INSERT INTO imports (title, full_text, status, depth, focus, created_at)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING id AS "id!""#,
            import.title,
            import.text,
            status,
            depth,
            import.focus,
            created_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(repo_error)
    }

    async fn generation_request(&self, import: ImportId) -> Result<GenerationRequest, RepoError> {
        let row = sqlx::query!(
            "SELECT title, full_text, depth, focus FROM imports WHERE id = ?",
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
        })
    }

    async fn mark_generating(&self, import: ImportId) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE imports SET status = 'generating', error = NULL WHERE id = ?",
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
        cards: &[GeneratedCard],
    ) -> Result<(), RepoError> {
        let mut transaction: SqliteTransaction<'static> = self.begin().await?;
        for card in cards {
            let id: i64 = sqlx::query_scalar!(
                r#"INSERT INTO cards (question, answer, status, created_at)
                   VALUES (?, ?, 'draft', (SELECT created_at FROM imports WHERE id = ?))
                   RETURNING id AS "id!""#,
                card.question,
                card.answer,
                import
            )
            .fetch_one(&mut *transaction)
            .await
            .map_err(repo_error)?;

            sqlx::query!(
                "INSERT INTO origins (card_id, import_id, source_title, excerpt, imported_at)
                 SELECT ?, id, title, ?, created_at FROM imports WHERE id = ?",
                id,
                card.excerpt,
                import
            )
            .execute(&mut *transaction)
            .await
            .map_err(repo_error)?;
        }
        sqlx::query!(
            "UPDATE imports SET status = 'in_inbox', error = NULL WHERE id = ?",
            import
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?;
        transaction.commit().await.map_err(repo_error)
    }

    async fn mark_failed(&self, import: ImportId, message: &str) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE imports SET status = 'failed', error = ? WHERE id = ?",
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
            r#"SELECT id AS "id!", title, status, error, full_text FROM imports
               WHERE status IN ('generating', 'in_inbox', 'failed')
               ORDER BY id"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(repo_error)?;

        let mut imports: Vec<InboxImport> = Vec::with_capacity(rows.len());
        for row in rows {
            let drafts: Vec<DraftCard> = sqlx::query_as!(
                DraftCard,
                r#"SELECT c.id AS "id!", c.question, c.answer, o.excerpt AS "excerpt?"
                   FROM cards c
                   JOIN origins o ON o.card_id = c.id
                   WHERE c.status = 'draft' AND o.import_id = ?
                   ORDER BY c.id"#,
                row.id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(repo_error)?;

            imports.push(InboxImport {
                id: row.id,
                title: row.title,
                status: ImportStatus::parse(&row.status),
                error: row.error,
                text: row.full_text,
                drafts,
            });
        }
        Ok(imports)
    }

    async fn drafts_of(&self, import: ImportId) -> Result<Vec<CardId>, RepoError> {
        sqlx::query_scalar!(
            r#"SELECT c.id AS "id!" FROM cards c
               JOIN origins o ON o.card_id = c.id
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
            "UPDATE cards SET status = 'active' WHERE id = ? AND status = 'draft'",
            card
        )
        .execute(&mut *transaction)
        .await
        .map_err(repo_error)?
        .rows_affected();
        // Already accepted: the card may have review progress that must not be reset.
        if accepted > 0 {
            save_schedule(&mut transaction, card, schedule).await?;
        }
        let import: ImportId =
            sqlx::query_scalar!("SELECT import_id FROM origins WHERE card_id = ?", card)
                .fetch_one(&mut *transaction)
                .await
                .map_err(repo_error)?;
        transaction.commit().await.map_err(repo_error)?;
        Ok(import)
    }

    async fn update_draft(&self, card: CardId, content: &Card) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE cards SET question = ?, answer = ? WHERE id = ? AND status = 'draft'",
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
            sqlx::query_scalar!("SELECT import_id FROM origins WHERE card_id = ?", card)
                .fetch_one(&self.pool)
                .await
                .map_err(repo_error)?;
        sqlx::query!("DELETE FROM cards WHERE id = ? AND status = 'draft'", card)
            .execute(&self.pool)
            .await
            .map_err(repo_error)?;
        Ok(import)
    }

    async fn finish_import_if_empty(&self, import: ImportId) -> Result<(), RepoError> {
        sqlx::query!(
            "UPDATE imports SET status = 'done', full_text = NULL
             WHERE id = ? AND status = 'in_inbox' AND NOT EXISTS (
                 SELECT 1 FROM origins o JOIN cards c ON c.id = o.card_id
                 WHERE o.import_id = imports.id AND c.status = 'draft'
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
            "DELETE FROM imports WHERE id = ? AND status = 'failed'",
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
               FROM imports i
               LEFT JOIN origins o ON o.import_id = i.id
               LEFT JOIN cards c ON c.id = o.card_id
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
        sqlx::query_scalar!("SELECT COUNT(*) FROM cards WHERE status = 'draft'")
            .fetch_one(&self.pool)
            .await
            .map_err(repo_error)
    }
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
