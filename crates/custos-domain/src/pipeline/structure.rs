//! Tabular Structure Detection & Data Contract Invariants
//!
//! Provides automated heuristic detection for messy tabular feeds (CSV, TSV, DSV),
//! column type inference, header row identification, and schema quality verification.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use super::DataInvariant;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferredDataType {
    Integer,
    Float,
    Datetime,
    Boolean,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferredColumn {
    pub name: String,
    pub data_type: InferredDataType,
    pub null_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectedStructure {
    pub delimiter: char,
    pub header_row_index: usize,
    pub headers: Vec<String>,
    pub columns: Vec<InferredColumn>,
    pub total_rows: usize,
    pub quality_score: f64,
}

/// Heuristic tabular structure detector for raw incoming unstructured text/CSV
pub struct TabularStructureDetector;

impl TabularStructureDetector {
    /// Detect optimal delimiter among comma, tab, semicolon, and pipe.
    pub fn detect_delimiter(sample: &str) -> char {
        let candidates = [',', '\t', ';', '|'];
        let lines: Vec<&str> = sample.lines().filter(|l| !l.trim().is_empty()).take(10).collect();
        if lines.is_empty() {
            return ',';
        }

        let mut best_delim = ',';
        let mut best_score = -1.0;

        for &delim in &candidates {
            let counts: Vec<usize> = lines.iter().map(|l| l.matches(delim).count()).collect();
            if counts.is_empty() || counts[0] == 0 {
                continue;
            }
            let first = counts[0];
            let consistent = counts.iter().all(|&c| c == first);
            let score = if consistent { first as f64 * 10.0 } else { 0.0 };
            if score > best_score {
                best_score = score;
                best_delim = delim;
            }
        }

        best_delim
    }

    /// Split raw input using detected delimiter into rows and columns.
    pub fn parse_rows(text: &str, delimiter: char) -> Vec<Vec<String>> {
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split(delimiter).map(|s| s.trim().to_string()).collect())
            .collect()
    }

    /// Locate the true header row index by inspecting text vs numeric density and table width.
    pub fn find_header_row(rows: &[Vec<String>]) -> usize {
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        for (idx, row) in rows.iter().enumerate() {
            if row.is_empty() {
                continue;
            }
            // Title or banner metadata rows typically have fewer columns than the table
            if max_cols > 1 && row.len() < max_cols {
                continue;
            }
            let filled_count = row.iter().filter(|c| !c.is_empty()).count();
            if filled_count <= 1 && max_cols > 1 {
                continue;
            }

            // Headers are typically non-numeric text strings
            let text_count = row.iter().filter(|c| !c.is_empty() && c.parse::<f64>().is_err()).count();
            if text_count as f64 / filled_count.max(1) as f64 >= 0.7 {
                return idx;
            }
        }
        0
    }

    /// Detect structure and infer column types.
    pub fn detect(text: &str) -> Result<DetectedStructure, DomainError> {
        let delimiter = Self::detect_delimiter(text);
        let rows = Self::parse_rows(text, delimiter);
        if rows.is_empty() {
            return Err(DomainError::Validation("Empty tabular payload".into()));
        }

        let header_idx = Self::find_header_row(&rows);
        let headers: Vec<String> = rows[header_idx]
            .iter()
            .enumerate()
            .map(|(i, col)| if col.is_empty() { format!("column_{i}") } else { col.clone() })
            .collect();

        let data_rows = &rows[(header_idx + 1)..];
        let _num_cols = headers.len();
        let total_data_rows = data_rows.len();

        let mut columns = Vec::new();
        let mut total_cells = 0;
        let mut valid_cells = 0;

        for (col_idx, name) in headers.iter().enumerate() {
            let mut null_count = 0;
            let mut int_count = 0;
            let mut float_count = 0;
            let mut dt_count = 0;
            let mut bool_count = 0;
            let mut total_sample = 0;

            for r in data_rows.iter().take(200) {
                total_cells += 1;
                if let Some(val) = r.get(col_idx) {
                    let trimmed = val.trim();
                    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed.eq_ignore_ascii_case("nan") {
                        null_count += 1;
                    } else {
                        valid_cells += 1;
                        total_sample += 1;
                        if trimmed.parse::<i64>().is_ok() {
                            int_count += 1;
                        } else if trimmed.parse::<f64>().is_ok() {
                            float_count += 1;
                        } else if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
                            bool_count += 1;
                        } else if trimmed.contains('-') || trimmed.contains('/') || trimmed.contains(':') {
                            dt_count += 1;
                        }
                    }
                } else {
                    null_count += 1;
                }
            }

            let non_null = total_sample.max(1) as f64;
            let dtype = if int_count as f64 / non_null >= 0.85 {
                InferredDataType::Integer
            } else if (int_count + float_count) as f64 / non_null >= 0.85 {
                InferredDataType::Float
            } else if bool_count as f64 / non_null >= 0.85 {
                InferredDataType::Boolean
            } else if dt_count as f64 / non_null >= 0.70 {
                InferredDataType::Datetime
            } else {
                InferredDataType::Text
            };

            let null_ratio = if total_data_rows > 0 {
                null_count as f64 / total_data_rows.min(200) as f64
            } else {
                0.0
            };

            columns.push(InferredColumn {
                name: name.clone(),
                data_type: dtype,
                null_ratio,
            });
        }

        let quality_score = if total_cells > 0 {
            valid_cells as f64 / total_cells as f64
        } else {
            1.0
        };

        Ok(DetectedStructure {
            delimiter,
            header_row_index: header_idx,
            headers,
            columns,
            total_rows: total_data_rows,
            quality_score,
        })
    }
}

/// Invariant enforcing minimum column presence and quality score thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularContractInvariant {
    pub required_columns: Vec<String>,
    pub min_quality_score: f64,
    pub detected: DetectedStructure,
}

impl TabularContractInvariant {
    pub fn new(required_columns: Vec<String>, min_quality_score: f64, detected: DetectedStructure) -> Self {
        Self {
            required_columns,
            min_quality_score,
            detected,
        }
    }
}

impl DataInvariant for TabularContractInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.detected.quality_score < self.min_quality_score {
            return Err(DomainError::Validation(format!(
                "Tabular quality score {:.2} below required minimum {:.2}",
                self.detected.quality_score, self.min_quality_score
            )));
        }

        for req in &self.required_columns {
            let found = self.detected.headers.iter().any(|h| h.eq_ignore_ascii_case(req));
            if !found {
                return Err(DomainError::Validation(format!(
                    "Missing mandatory column '{}' in tabular contract",
                    req
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structure_detector_and_type_inference() {
        let csv_data = "REPORT TITLE: LAB RESULTS OCTOBER\nPID,Test_Value,Is_Active,Received_Date\nP1001,15.5,true,2026-10-01 08:30:00\nP1002,18.2,false,2026-10-01 09:15:00\nP1003,12.0,true,2026-10-01 10:00:00";

        let detected = TabularStructureDetector::detect(csv_data).expect("Detection should succeed");
        assert_eq!(detected.delimiter, ',');
        assert_eq!(detected.header_row_index, 1);
        assert_eq!(detected.headers, vec!["PID", "Test_Value", "Is_Active", "Received_Date"]);
        assert_eq!(detected.columns[0].data_type, InferredDataType::Text);
        assert_eq!(detected.columns[1].data_type, InferredDataType::Float);
        assert_eq!(detected.columns[2].data_type, InferredDataType::Boolean);
        assert_eq!(detected.columns[3].data_type, InferredDataType::Datetime);
        assert!(detected.quality_score > 0.95);

        // Contract Invariant verification
        let invariant_pass = TabularContractInvariant::new(
            vec!["PID".into(), "Test_Value".into()],
            0.9,
            detected.clone(),
        );
        assert!(invariant_pass.verify().is_ok());

        let invariant_fail = TabularContractInvariant::new(
            vec!["PID".into(), "NonExistentColumn".into()],
            0.9,
            detected,
        );
        assert!(invariant_fail.verify().is_err());
    }
}
