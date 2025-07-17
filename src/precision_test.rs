#[cfg(test)]
mod precision_tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// f64의 정밀도 문제를 보여주는 테스트
    #[test]
    fn test_f64_precision_loss() {
        // 1. 기본적인 부동소수점 오차
        let price1 = 0.1 + 0.2;
        assert_ne!(price1, 0.3); // 0.30000000000000004
        println!("0.1 + 0.2 = {:.20}", price1);

        // 2. BTC 가격에서의 정밀도 손실
        let btc_price: f64 = 65432.123456789;
        let small_change: f64 = 0.000000001; // 1 satoshi
        let new_price = btc_price + small_change;

        // 작은 변화가 무시될 수 있음 확인
        println!("BTC price: {:.20}", btc_price);
        println!("New price: {:.20}", new_price);
        println!("Are they equal? {}", btc_price == new_price);
        println!("Difference: {:.20}", (new_price - btc_price).abs());

        // 3. 큰 숫자에서의 정밀도 문제
        let large_amount: f64 = 100_000_000.0; // 1억
        let small_fee: f64 = 0.00001; // 작은 수수료
        let result = large_amount + small_fee;

        // 수수료가 사라질 수 있음
        println!(
            "Large amount + small fee: {} + {} = {:.10}",
            large_amount, small_fee, result
        );
    }

    /// 반올림으로 인한 손실 테스트
    #[test]
    fn test_rounding_errors() {
        // 여러 번의 거래로 누적되는 오차
        let mut balance: f64 = 1.0;
        let fee: f64 = 0.001; // 0.1% 수수료

        // 1000번의 작은 거래
        for _ in 0..1000 {
            let amount = 0.001;
            balance -= amount;
            balance -= amount * fee;
        }

        // 예상: 1.0 - 1.0 - 0.001 = -0.001
        // 실제: 부동소수점 오차로 인해 다를 수 있음
        println!("After 1000 transactions: {:.20}", balance);

        // 정확한 계산
        let expected = 1.0 - 1.0 - 0.001;
        let difference = (balance - expected).abs();
        println!("Error accumulated: {:.20}", difference);
    }

    /// Decimal을 사용한 정밀한 계산
    #[test]
    fn test_decimal_precision() {
        // Decimal을 사용하면 정확한 계산 가능
        let price1 = Decimal::from_str("0.1").unwrap();
        let price2 = Decimal::from_str("0.2").unwrap();
        let sum = price1 + price2;

        assert_eq!(sum.to_string(), "0.3"); // 정확히 0.3

        // BTC 가격 정밀도 테스트
        let btc_price = Decimal::from_str("65432.123456789").unwrap();
        let satoshi = Decimal::from_str("0.000000001").unwrap();
        let new_price = btc_price + satoshi;

        // 작은 변화도 정확히 반영
        assert_ne!(btc_price, new_price);
        println!(
            "Decimal BTC price: {} + {} = {}",
            btc_price, satoshi, new_price
        );
    }

    /// 실제 거래소 가격 시뮬레이션
    #[test]
    fn test_exchange_price_aggregation() {
        // 여러 거래소의 가격 (f64 사용 시)
        let prices_f64 = vec![65432.123456789, 65432.123456788, 65432.123456790];

        let avg_f64: f64 = prices_f64.iter().sum::<f64>() / prices_f64.len() as f64;
        println!("Average price (f64): {:.20}", avg_f64);

        // Decimal 사용 시
        let prices_decimal: Vec<Decimal> = vec![
            Decimal::from_str("65432.123456789").unwrap(),
            Decimal::from_str("65432.123456788").unwrap(),
            Decimal::from_str("65432.123456790").unwrap(),
        ];

        let sum: Decimal = prices_decimal.iter().sum();
        let avg_decimal = sum / Decimal::from(prices_decimal.len());
        println!("Average price (Decimal): {}", avg_decimal);

        // 차이 비교
        let diff = Decimal::from_f64_retain(avg_f64).unwrap() - avg_decimal;
        println!("Difference: {}", diff.abs());
    }

    /// 사토시 단위 변환 테스트
    #[test]
    fn test_satoshi_conversion() {
        // 1 BTC = 100,000,000 satoshi
        let btc_f64: f64 = 1.0;
        let satoshis_f64 = btc_f64 * 100_000_000.0;

        // 작은 금액 테스트
        let small_btc: f64 = 0.00000001; // 1 satoshi
        let small_satoshis = small_btc * 100_000_000.0;

        // f64로는 정확하지 않을 수 있음
        assert_eq!(small_satoshis, 1.0); // 이것도 정확하지 않을 수 있음

        // Decimal 사용
        let btc_decimal = Decimal::from_str("0.00000001").unwrap();
        let satoshis_decimal = btc_decimal * Decimal::from(100_000_000);
        assert_eq!(satoshis_decimal, Decimal::from(1));

        println!("1 satoshi in BTC (f64): {:.20}", small_btc);
        println!("1 satoshi in BTC (Decimal): {}", btc_decimal);
    }

    /// 권장사항 테스트
    #[test]
    fn test_recommended_approach() {
        // 권장: 정수로 저장 (satoshi 단위)
        let price_in_satoshis: u64 = 6543212345678; // 65432.12345678 BTC

        // 표시할 때만 BTC로 변환
        let btc_for_display = price_in_satoshis as f64 / 100_000_000.0;
        println!(
            "Price: {} satoshis = {:.8} BTC",
            price_in_satoshis, btc_for_display
        );

        // 계산은 정수로
        let fee_in_satoshis: u64 = 10000; // 0.0001 BTC
        let total = price_in_satoshis + fee_in_satoshis;

        println!("Total: {} satoshis", total);
    }
}

/// 프로덕션 코드에서 사용할 안전한 가격 처리
pub mod safe_price {
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    /// 안전한 BTC 가격 구조체
    #[derive(Debug, Clone, PartialEq)]
    pub struct BtcPrice {
        satoshis: u64,
    }

    impl BtcPrice {
        /// satoshi 단위로 생성
        pub fn from_satoshis(satoshis: u64) -> Self {
            Self { satoshis }
        }

        /// BTC 문자열에서 생성
        pub fn from_btc_str(btc: &str) -> Result<Self, String> {
            let decimal =
                Decimal::from_str(btc).map_err(|e| format!("Invalid BTC amount: {}", e))?;

            let satoshis_decimal = decimal * Decimal::from(100_000_000);
            let satoshis = satoshis_decimal
                .to_u64()
                .ok_or_else(|| "BTC amount too large".to_string())?;

            Ok(Self { satoshis })
        }

        /// BTC로 표시 (표시용)
        pub fn to_btc_f64(&self) -> f64 {
            self.satoshis as f64 / 100_000_000.0
        }

        /// 정확한 BTC 문자열
        pub fn to_btc_string(&self) -> String {
            let decimal = Decimal::from(self.satoshis) / Decimal::from(100_000_000);
            decimal.to_string()
        }
    }
}
