use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::card::RepoError;

const LOWEST_RETENTION: u32 = 80;
const HIGHEST_RETENTION: u32 = 97;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

/// How large the whole app is drawn, for big screens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    #[default]
    Default,
    Larger,
    Largest,
}

impl Size {
    pub fn as_str(self) -> &'static str {
        match self {
            Size::Default => "default",
            Size::Larger => "larger",
            Size::Largest => "largest",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "larger" => Size::Larger,
            "largest" => Size::Largest,
            _ => Size::Default,
        }
    }

    /// How much the window zooms everything: text, spacing and the sidebar alike.
    pub fn zoom(self) -> f64 {
        match self {
            Size::Default => 1.3,
            Size::Larger => 1.45,
            Size::Largest => 1.6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub new_cards_per_day: u32,
    pub desired_retention_percent: u32,
    pub theme: Theme,
    pub size: Size,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_cards_per_day: 20,
            desired_retention_percent: 90,
            theme: Theme::System,
            size: Size::Default,
        }
    }
}

impl Settings {
    /// Keeps retention in the range FSRS is sensible for.
    pub fn validated(self) -> Self {
        Self {
            desired_retention_percent: self
                .desired_retention_percent
                .clamp(LOWEST_RETENTION, HIGHEST_RETENTION),
            ..self
        }
    }

    pub fn desired_retention(&self) -> f64 {
        f64::from(self.desired_retention_percent) / 100.0
    }
}

/// Port for storing settings.
pub trait SettingsRepo {
    fn load_settings(&self) -> impl Future<Output = Result<Settings, RepoError>> + Send;

    fn save_settings(
        &self,
        settings: &Settings,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;
}
