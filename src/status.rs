//! Data structures for reporting transaction statuses.

use crate::solana::SigResponse;
use crate::transaction::Base64;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{cmp::Eq, fmt, hash::Hash, path::PathBuf};

const STRFTIME: &str = "%Y-%m-%d %H:%M:%S";

/// Status as reported directly from the network.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RawStatus {
    pub block_height: u64,
    pub block_indep_hash: Base64,
    pub number_of_confirmations: u64,
}

/// Indicates transaction status on the network, from Submitted to Confirmed.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone, Eq, Hash)]
pub enum StatusCode {
    #[default]
    Submitted,
    Pending,
    Confirmed,
    NotFound,
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StatusCode::Submitted => write!(f, "Submitted"),
            StatusCode::Pending => write!(f, "Pending"),
            StatusCode::Confirmed => write!(f, "Confirmed"),
            StatusCode::NotFound => write!(f, "NotFound"),
        }
    }
}
pub struct FilterElements<'a> {
    pub raw_status: &'a Option<RawStatus>,
    pub status: &'a StatusCode,
}
pub trait Filterable {
    fn get_filter_elements(&self) -> FilterElements;
}

/// Data structure for tracking transaction statuses.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Status {
    pub id: Base64,
    pub status: StatusCode,
    pub file_path: Option<PathBuf>,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub reward: u64,
    #[serde(flatten)]
    pub raw_status: Option<RawStatus>,
    #[serde(flatten)]
    pub sol_sig: Option<SigResponse>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            id: Base64(vec![]),
            status: StatusCode::default(),
            file_path: None,
            content_type: mime_guess::mime::OCTET_STREAM.to_string(),
            created_at: Utc::now(),
            last_modified: Utc::now(),
            reward: 0,
            raw_status: None,
            sol_sig: None,
        }
    }
}

impl Status {
    pub fn header_string(&self, output_format: &OutputFormat) -> String {
        match output_format {
            OutputFormat::Display => {
                format!(
                    " {:<30}  {:<43}  {:<9}  {}\n{:-<97}",
                    "path", "id", "status", "confirms", ""
                )
            }
            _ => format!("{}", ""),
        }
    }
}

impl Filterable for Status {
    fn get_filter_elements(&self) -> FilterElements {
        FilterElements {
            raw_status: &self.raw_status,
            status: &self.status,
        }
    }
}

impl QuietDisplay for Status {
    fn write_str(&self, _w: &mut dyn fmt::Write) -> fmt::Result {
        Ok(())
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            " {:<30}  {:<43}  {:<9}  {:>8}",
            self.file_path
                .as_ref()
                .map(|f| f.display().to_string())
                .unwrap_or("".to_string()),
            self.id,
            self.status.to_string(),
            self.raw_status
                .as_ref()
                .map(|f| f.number_of_confirmations)
                .unwrap_or(0)
                .to_string(),
        )
    }
}

impl VerboseDisplay for Status {
    fn write_str(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        writeln!(w, "{:<15} {}", "id:", self.id)?;
        writeln!(w, "{:<15} {:?}", "status:", self.status)?;
        if let Some(file_path) = &self.file_path {
            writeln!(
                w,
                "{:<15} {}",
                "file_path:",
                file_path.display().to_string()
            )?;
        };
        writeln!(
            w,
            "{:<15} {}",
            "created_at:",
            self.created_at.format(STRFTIME).to_string()
        )?;
        writeln!(
            w,
            "{:<15} {}",
            "last_modified:",
            self.last_modified.format(STRFTIME).to_string()
        )?;
        if let Some(raw_status) = &self.raw_status {
            writeln!(w, "{:<15} {}", "height:", raw_status.block_height)?;
            writeln!(w, "{:<15} {}", "indep_hash:", raw_status.block_indep_hash)?;
            writeln!(
                w,
                "{:<15} {}",
                "confirms:", raw_status.number_of_confirmations
            )?;
        };
        writeln!(w, "")
    }
}

/// Data structure for tracking bundle statuses.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct BundleStatus {
    pub id: Base64,
    pub status: StatusCode,
    pub file_paths: Value,
    pub number_of_files: u64,
    pub data_size: u64,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub reward: u64,
    #[serde(flatten)]
    pub raw_status: Option<RawStatus>,
    #[serde(flatten)]
    pub sol_sig: Option<SigResponse>,
}

impl Default for BundleStatus {
    fn default() -> Self {
        Self {
            id: Base64(vec![]),
            status: StatusCode::default(),
            file_paths: json!({}),
            number_of_files: 0,
            data_size: 0,
            created_at: Utc::now(),
            last_modified: Utc::now(),
            reward: 0,
            raw_status: None,
            sol_sig: None,
        }
    }
}

impl BundleStatus {
    pub fn header_string(&self, output_format: &OutputFormat) -> String {
        match output_format {
            OutputFormat::Display => {
                format!(
                    " {:<43}  {:>6}  {:>6}  {:<11}  {}\n{:-<84}",
                    "bundle txid", "items", "KB", "status", "confirms", ""
                )
            }
            _ => format!("{}", ""),
        }
    }
}

impl Filterable for BundleStatus {
    fn get_filter_elements(&self) -> FilterElements {
        FilterElements {
            raw_status: &self.raw_status,
            status: &self.status,
        }
    }
}

impl QuietDisplay for BundleStatus {
    fn write_str(&self, _w: &mut dyn fmt::Write) -> fmt::Result {
        Ok(())
    }
}

impl std::fmt::Display for BundleStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            " {:<43}  {:>6}  {:>6}  {:<11} {:>9}",
            self.id,
            self.number_of_files,
            self.data_size / 1000,
            self.status.to_string(),
            self.raw_status
                .as_ref()
                .map(|f| f.number_of_confirmations)
                .unwrap_or(0)
                .to_string(),
        )
    }
}

impl VerboseDisplay for BundleStatus {
    fn write_str(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        writeln!(w, "{:<15} {}", "id:", self.id)?;
        writeln!(w, "{:<15} {:?}", "status:", self.status)?;
        writeln!(
            w,
            "{:<15} {}",
            "created_at:",
            self.created_at.format(STRFTIME).to_string()
        )?;
        writeln!(
            w,
            "{:<15} {}",
            "last_modified:",
            self.last_modified.format(STRFTIME).to_string()
        )?;
        if let Some(raw_status) = &self.raw_status {
            writeln!(w, "{:<15} {}", "height:", raw_status.block_height)?;
            writeln!(w, "{:<15} {}", "indep_hash:", raw_status.block_indep_hash)?;
            writeln!(
                w,
                "{:<15} {}",
                "confirms:", raw_status.number_of_confirmations
            )?;
        };
        writeln!(w, "")
    }
}

/// Controls output format, including quiet, verbose and json formats.
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Display,
    Json,
    JsonCompact,
    DisplayQuiet,
    DisplayVerbose,
}

impl OutputFormat {
    pub fn formatted_string<T>(&self, item: &T) -> String
    where
        T: Serialize + fmt::Display + QuietDisplay + VerboseDisplay,
    {
        match self {
            OutputFormat::Display => format!("{}", item),
            OutputFormat::DisplayQuiet => {
                let mut s = String::new();
                QuietDisplay::write_str(item, &mut s).unwrap();
                s
            }
            OutputFormat::DisplayVerbose => {
                let mut s = String::new();
                VerboseDisplay::write_str(item, &mut s).unwrap();
                s
            }
            OutputFormat::Json => {
                let mut string = serde_json::to_string_pretty(item).unwrap();
                ",\n".chars().for_each(|c| string.push(c));
                string
            }
            OutputFormat::JsonCompact => {
                let mut string = serde_json::to_value(item).unwrap().to_string();
                ",\n".chars().for_each(|c| string.push(c));
                string
            }
        }
    }
}

/// Implements header for output with multiple records.
pub trait OutputHeader<T> {
    fn header_string(output_format: &OutputFormat) -> String
    where
        T: Serialize + fmt::Display + QuietDisplay + VerboseDisplay;
}

/// Implements output for quiet display output format.
pub trait QuietDisplay: fmt::Display {
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(w, "{}", self)
    }
}

/// Implements output for verbose display output format.
pub trait VerboseDisplay: fmt::Display {
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        write!(w, "{}", self)
    }
}
