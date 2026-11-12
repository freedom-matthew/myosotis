//! Concepts were once a term and its meaning, swapped round for the other direction, so the
//! front was a bare term. This rewords those old pairs as two full questions, once, keeping
//! their review history.

use std::path::PathBuf;

use chrono::Utc;
use myosotis_core::{
    Card, CardKind, CardRepo, CardRewriter, GenerateError, OldConcept, RepoError, Reverse,
    RewriteRequest,
};

use crate::claude_code::ClaudeCode;
use crate::features::backup;
use crate::storage::SqliteCardRepo;

pub async fn has_old(repo: &impl CardRepo) -> Result<bool, RepoError> {
    Ok(!repo.old_concepts().await?.is_empty())
}

/// Rewords every old concept and returns how many. Stops at the first failure, leaving the
/// rest as they were for the next try.
pub async fn reword_old(
    repo: &impl CardRepo,
    rewriter: &impl CardRewriter,
) -> Result<usize, GenerateError> {
    let old: Vec<OldConcept> = repo.old_concepts().await.map_err(storage_failed)?;
    let mut reworded: usize = 0;
    for concept in &old {
        let request: RewriteRequest = RewriteRequest {
            kind: CardKind::Concept,
            question: concept.term.clone(),
            answer: concept.meaning.clone(),
            reverse: None,
            excerpt: concept.excerpt.clone(),
        };
        let better: Card = rewriter.rewrite(&request).await?;
        let Some(reverse): Option<Reverse> = better.reverse.clone() else {
            continue;
        };
        repo.reword_concept(concept, &better, &reverse)
            .await
            .map_err(storage_failed)?;
        reworded += 1;
    }
    Ok(reworded)
}

/// Runs in the background after startup: backs up first, then rewords. A failure (say,
/// Claude Code isn't signed in) waits for the next start.
pub async fn reword_after_startup(repo: SqliteCardRepo, backups: PathBuf) {
    match has_old(&repo).await {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => return eprintln!("could not look for old concepts: {error}"),
    }
    if let Err(error) = backup::back_up(&repo, &backups, Utc::now()).await {
        return eprintln!("not rewording concepts, the backup failed: {error}");
    }
    if let Err(error) = reword_old(&repo, &ClaudeCode).await {
        eprintln!("rewording concepts stopped: {error}");
    }
}

fn storage_failed(error: RepoError) -> GenerateError {
    GenerateError(error.to_string())
}
