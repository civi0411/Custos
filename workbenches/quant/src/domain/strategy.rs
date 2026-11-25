//! Zero-I/O Quantitative Strategy, Position & Market Identity Models
//!
//! Enforces explicit market categories (never inferring market type from symbol string).

use custos_contracts::{digest, new_id, DomainError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketCategory {
    Crypto,
    USStock,
    HKStock,
    CNStock,
    Forex,
    Futures,
    MOEX,
}

impl MarketCategory {
    pub fn parse_explicit(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "crypto" | "cryptocurrency" => Ok(Self::Crypto),
            "usstock" | "us_stock" | "us_stocks" | "equity" | "equities" => Ok(Self::USStock),
            "hkstock" | "hk_stock" | "hk" => Ok(Self::HKStock),
            "cnstock" | "cn_stock" | "ashare" | "a_share" => Ok(Self::CNStock),
            "forex" | "fx" => Ok(Self::Forex),
            "futures" | "future" => Ok(Self::Futures),
            "moex" => Ok(Self::MOEX),
            other => Err(DomainError::Validation(format!(
                "Unsupported market category '{other}'; specify one of Crypto, USStock, HKStock, CNStock, Forex, Futures, MOEX"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Crypto => "Crypto",
            Self::USStock => "USStock",
            Self::HKStock => "HKStock",
            Self::CNStock => "CNStock",
            Self::Forex => "Forex",
            Self::Futures => "Futures",
            Self::MOEX => "MOEX",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionMode {
    LongOnly,
    ShortOnly,
    Bidirectional,
}

impl DirectionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LongOnly => "long_only",
            Self::ShortOnly => "short_only",
            Self::Bidirectional => "bidirectional",
        }
    }

    pub fn from_str_loose(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "short_only" | "short" => Self::ShortOnly,
            "bidirectional" | "both" | "long_short" => Self::Bidirectional,
            _ => Self::LongOnly,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantStrategyRecord {
    pub id: String,
    pub name: String,
    pub market: String,
    pub venue: String,
    pub symbol: String,
    pub timeframe: String,
    pub formula: String,
    pub direction_mode: String,
    pub status: String,
    pub latest_sharpe: Option<f64>,
    pub latest_ic_mean: Option<f64>,
    pub source_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl QuantStrategyRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<String>,
        name: impl Into<String>,
        market: MarketCategory,
        venue: impl Into<String>,
        symbol: impl Into<String>,
        timeframe: impl Into<String>,
        formula: impl Into<String>,
        direction_mode: DirectionMode,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let symbol = symbol.into().trim().to_uppercase();
        let formula = formula.into();
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Strategy name cannot be empty".into(),
            ));
        }
        if symbol.is_empty() {
            return Err(DomainError::Validation(
                "Strategy symbol cannot be empty".into(),
            ));
        }
        if formula.trim().is_empty() {
            return Err(DomainError::Validation(
                "Strategy formula cannot be empty".into(),
            ));
        }
        crate::domain::detect_lookahead_bias(&formula)?;

        let now = chrono::Utc::now().timestamp();
        let source_hash = format!("sha256:{}", digest(formula.as_bytes()));
        Ok(Self {
            id: id.unwrap_or_else(|| new_id("strat")),
            name,
            market: market.as_str().to_string(),
            venue: venue.into(),
            symbol,
            timeframe: timeframe.into(),
            formula,
            direction_mode: direction_mode.as_str().to_string(),
            status: "backtest".to_string(),
            latest_sharpe: None,
            latest_ic_mean: None,
            source_hash,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantPositionRecord {
    pub id: String,
    pub strategy_id: String,
    pub symbol: String,
    pub market: String,
    pub direction: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub realized_pnl: f64,
    pub updated_at: i64,
}
