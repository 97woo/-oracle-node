use anyhow::{Context, Result};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use oracle_vm_common::types::PriceData;

/// 안전한 BTC 가격 처리를 위한 래퍼 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SafeBtcPrice {
    satoshis: u64,
}

impl SafeBtcPrice {
    /// satoshi 단위로 새 가격 생성
    pub fn from_satoshis(satoshis: u64) -> Self {
        Self { satoshis }
    }

    /// BTC 문자열에서 생성 (예: "65432.12345678")
    pub fn from_btc_str(btc_str: &str) -> Result<Self> {
        let decimal = Decimal::from_str(btc_str).context("Invalid BTC price format")?;

        // 음수 체크
        if decimal.is_sign_negative() {
            anyhow::bail!("BTC price cannot be negative");
        }

        // satoshi로 변환 (1 BTC = 100,000,000 satoshis)
        let satoshis_decimal = decimal * Decimal::from(100_000_000);

        // 정수로 변환
        let satoshis = satoshis_decimal
            .to_u64()
            .ok_or_else(|| anyhow::anyhow!("BTC price too large"))?;

        Ok(Self { satoshis })
    }

    /// f64에서 생성 (권장하지 않음, 정밀도 손실 가능)
    #[deprecated(note = "Use from_btc_str for precise conversion")]
    pub fn from_f64(btc: f64) -> Result<Self> {
        if btc < 0.0 {
            anyhow::bail!("BTC price cannot be negative");
        }

        let satoshis = (btc * 100_000_000.0).round() as u64;
        Ok(Self { satoshis })
    }

    /// USD 가격에서 생성 (TDD 테스트용)
    pub fn from_price(price_usd: f64) -> Self {
        let satoshis = (price_usd * 100_000_000.0).round() as u64;
        Self { satoshis }
    }

    /// satoshi 값 반환
    pub fn as_satoshis(&self) -> u64 {
        self.satoshis
    }

    /// BTC로 표시 (표시용, 계산에 사용하지 말 것)
    pub fn to_btc_display(&self) -> f64 {
        self.satoshis as f64 / 100_000_000.0
    }

    /// USD 가격으로 반환 (as_price 메서드)
    pub fn as_price(&self) -> f64 {
        self.satoshis as f64 / 100_000_000.0
    }

    /// 정확한 BTC 문자열 반환
    pub fn to_btc_string(&self) -> String {
        let decimal = Decimal::from(self.satoshis) / Decimal::from(100_000_000);
        decimal.to_string()
    }

    /// 두 가격의 차이 (satoshi 단위)
    pub fn difference(&self, other: &Self) -> i64 {
        self.satoshis as i64 - other.satoshis as i64
    }

    /// 퍼센트 차이 계산
    pub fn percent_difference(&self, other: &Self) -> f64 {
        if other.satoshis == 0 {
            return 0.0;
        }

        let diff = self.satoshis as f64 - other.satoshis as f64;
        (diff / other.satoshis as f64) * 100.0
    }

    /// 두 가격을 더함
    pub fn add(&self, other: &Self) -> Self {
        Self {
            satoshis: self.satoshis + other.satoshis,
        }
    }

    /// 두 가격을 뺌 (underflow 방지)
    pub fn subtract(&self, other: &Self) -> Result<Self> {
        if self.satoshis < other.satoshis {
            anyhow::bail!("Subtraction would result in negative value");
        }
        Ok(Self {
            satoshis: self.satoshis - other.satoshis,
        })
    }

    /// 수량을 곱함
    pub fn multiply(&self, quantity: f64) -> Self {
        Self {
            satoshis: (self.satoshis as f64 * quantity).round() as u64,
        }
    }

    /// 수량으로 나눔
    pub fn divide(&self, quantity: f64) -> Self {
        Self {
            satoshis: (self.satoshis as f64 / quantity).round() as u64,
        }
    }

    /// 퍼센트 적용 (예: 1.0 = 1% 증가, -1.0 = 1% 감소)
    pub fn apply_percentage(&self, percent: f64) -> Self {
        let factor = 1.0 + (percent / 100.0);
        Self {
            satoshis: (self.satoshis as f64 * factor).round() as u64,
        }
    }

    /// 여러 가격의 평균 계산
    pub fn average(prices: &[Self]) -> Option<Self> {
        if prices.is_empty() {
            return None;
        }
        let sum: u64 = prices.iter().map(|p| p.satoshis).sum();
        Some(Self {
            satoshis: sum / prices.len() as u64,
        })
    }

    /// 여러 가격의 중간값 계산
    pub fn median(prices: &[Self]) -> Option<Self> {
        if prices.is_empty() {
            return None;
        }
        let mut sorted = prices.to_vec();
        sorted.sort_by_key(|p| p.satoshis);
        
        let len = sorted.len();
        if len % 2 == 0 {
            // 짝수 개인 경우 중간 두 값의 평균
            let mid1 = sorted[len / 2 - 1];
            let mid2 = sorted[len / 2];
            Some(Self {
                satoshis: (mid1.satoshis + mid2.satoshis) / 2,
            })
        } else {
            // 홀수 개인 경우 중간값
            Some(sorted[len / 2])
        }
    }

    /// USD 형식으로 포맷
    pub fn format_usd(&self) -> String {
        let price = self.as_price();
        let integer_part = price.trunc() as i64;
        let decimal_part = ((price.fract() * 100.0).round() as i64).abs();
        
        // 천 단위 구분자 추가
        let formatted_integer = format!("{}", integer_part)
            .chars()
            .rev()
            .enumerate()
            .flat_map(|(i, c)| {
                if i > 0 && i % 3 == 0 {
                    vec![',', c]
                } else {
                    vec![c]
                }
            })
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        
        format!("${}.{:02}", formatted_integer, decimal_part)
    }
}

impl std::fmt::Display for SafeBtcPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.8} BTC", self.as_price())
    }
}

/// 기존 PriceData와 호환되는 안전한 버전
#[derive(Clone)]
pub struct SafePriceData {
    pub price: SafeBtcPrice,
    pub timestamp: u64,
    pub source: String,
}

impl SafePriceData {
    /// 기존 PriceData에서 변환
    pub fn from_price_data(data: &PriceData) -> Result<Self> {
        // Convert cents to dollars
        let price_usd = data.price as f64 / 100.0;
        #[allow(deprecated)]
        let safe_price = SafeBtcPrice::from_f64(price_usd)?;

        Ok(Self {
            price: safe_price,
            timestamp: data.timestamp.timestamp() as u64,
            source: data.source.clone(),
        })
    }

    /// 표시용 가격 (BTC)
    pub fn price_btc_display(&self) -> f64 {
        self.price.to_btc_display()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_btc_price_conversion() {
        // 문자열에서 생성
        let price = SafeBtcPrice::from_btc_str("65432.12345678").unwrap();
        assert_eq!(price.as_satoshis(), 6543212345678);
        assert_eq!(price.to_btc_string(), "65432.12345678");

        // 작은 값
        let small = SafeBtcPrice::from_btc_str("0.00000001").unwrap();
        assert_eq!(small.as_satoshis(), 1);

        // 큰 값
        let large = SafeBtcPrice::from_btc_str("21000000").unwrap();
        assert_eq!(large.as_satoshis(), 2100000000000000);
    }

    #[test]
    fn test_price_comparison() {
        let price1 = SafeBtcPrice::from_btc_str("100.0").unwrap();
        let price2 = SafeBtcPrice::from_btc_str("100.00000001").unwrap();

        assert!(price2 > price1);
        assert_eq!(price2.difference(&price1), 1);

        // 퍼센트 차이
        let percent_diff = price2.percent_difference(&price1);
        assert!(percent_diff > 0.0);
    }

    #[test]
    fn test_precision_preservation() {
        let price_str = "65432.123456789";
        let price = SafeBtcPrice::from_btc_str(price_str).unwrap();

        // satoshi 변환은 8자리까지만 정확 (BTC의 최소 단위)
        assert_eq!(price.to_btc_string(), "65432.12345678");
    }
}
