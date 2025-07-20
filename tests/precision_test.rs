use oracle_node::safe_price::SafeBtcPrice;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_price_from_float_precision() {
        // Given - 다양한 소수점 가격들
        let test_cases = vec![
            (70000.0, 7_000_000_000_000),
            (70000.12345678, 7_000_012_345_678),
            (0.00000001, 1), // 1 satoshi
            (0.00000123, 123), // 123 satoshis
            (99999.99999999, 9_999_999_999_999),
        ];

        for (float_price, expected_sats) in test_cases {
            // When
            let safe_price = SafeBtcPrice::from_price(float_price);

            // Then
            assert_eq!(
                safe_price.as_satoshis(),
                expected_sats,
                "Failed for price: {}",
                float_price
            );
        }
    }

    #[test]
    fn test_safe_price_to_float_precision() {
        // Given - satoshi 값들
        let test_cases = vec![
            (7_000_000_000_000, 70000.0),
            (1, 0.00000001),
            (123, 0.00000123),
            (7_000_012_345_678, 70000.12345678),
        ];

        for (sats, expected_price) in test_cases {
            // When
            let safe_price = SafeBtcPrice::from_satoshis(sats);
            let float_price = safe_price.as_price();

            // Then
            assert!(
                (float_price - expected_price).abs() < 0.000000001,
                "Failed for sats: {}, got: {}, expected: {}",
                sats,
                float_price,
                expected_price
            );
        }
    }

    #[test]
    fn test_safe_price_arithmetic_operations() {
        // Given
        let price1 = SafeBtcPrice::from_price(70000.0);
        let price2 = SafeBtcPrice::from_price(500.0);

        // When - 덧셈
        let sum = price1.add(&price2);
        assert_eq!(sum.as_price(), 70500.0);

        // When - 뺄셈
        let diff = price1.subtract(&price2).unwrap();
        assert_eq!(diff.as_price(), 69500.0);

        // When - 곱셈 (수량)
        let doubled = price1.multiply(2.0);
        assert_eq!(doubled.as_price(), 140000.0);

        // When - 나눗셈 (수량)
        let halved = price1.divide(2.0);
        assert_eq!(halved.as_price(), 35000.0);
    }

    #[test]
    fn test_safe_price_prevents_underflow() {
        // Given
        let price1 = SafeBtcPrice::from_price(100.0);
        let price2 = SafeBtcPrice::from_price(200.0);

        // When - 음수가 되는 뺄셈 시도
        let result = price1.subtract(&price2);

        // Then - 오류 반환
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_price_comparison() {
        // Given
        let price1 = SafeBtcPrice::from_price(70000.0);
        let price2 = SafeBtcPrice::from_price(70000.0);
        let price3 = SafeBtcPrice::from_price(69999.99999999);

        // Then
        assert_eq!(price1, price2);
        assert_ne!(price1, price3);
        assert!(price1 > price3);
        assert!(price3 < price1);
    }

    #[test]
    fn test_safe_price_percentage_operations() {
        // Given
        let base_price = SafeBtcPrice::from_price(70000.0);

        // When - 1% 증가
        let increased = base_price.apply_percentage(1.0);
        assert_eq!(increased.as_price(), 70700.0);

        // When - 1% 감소
        let decreased = base_price.apply_percentage(-1.0);
        assert_eq!(decreased.as_price(), 69300.0);

        // When - 0.1% 증가 (정밀도 테스트)
        let small_increase = base_price.apply_percentage(0.1);
        assert_eq!(small_increase.as_price(), 70070.0);
    }

    #[test]
    fn test_safe_price_average_calculation() {
        // Given
        let prices = vec![
            SafeBtcPrice::from_price(70000.0),
            SafeBtcPrice::from_price(70100.0),
            SafeBtcPrice::from_price(70200.0),
        ];

        // When
        let average = SafeBtcPrice::average(&prices).unwrap();

        // Then
        assert_eq!(average.as_price(), 70100.0);
    }

    #[test]
    fn test_safe_price_median_calculation() {
        // Given - 홀수 개
        let prices_odd = vec![
            SafeBtcPrice::from_price(70000.0),
            SafeBtcPrice::from_price(70500.0),
            SafeBtcPrice::from_price(70200.0),
        ];

        // When
        let median_odd = SafeBtcPrice::median(&prices_odd).unwrap();

        // Then
        assert_eq!(median_odd.as_price(), 70200.0);

        // Given - 짝수 개
        let prices_even = vec![
            SafeBtcPrice::from_price(70000.0),
            SafeBtcPrice::from_price(70100.0),
            SafeBtcPrice::from_price(70200.0),
            SafeBtcPrice::from_price(70300.0),
        ];

        // When
        let median_even = SafeBtcPrice::median(&prices_even).unwrap();

        // Then
        assert_eq!(median_even.as_price(), 70150.0);
    }

    #[test]
    fn test_safe_price_formatting() {
        // Given
        let price = SafeBtcPrice::from_price(70123.45678901);

        // When
        let formatted = price.to_string();

        // Then
        assert_eq!(formatted, "70123.45678901 BTC");

        // When - USD formatting
        let usd_formatted = price.format_usd();

        // Then
        assert_eq!(usd_formatted, "$70,123.46");
    }

    #[test]
    fn test_safe_price_serialization() {
        // Given
        let price = SafeBtcPrice::from_price(70000.12345678);

        // When - JSON 직렬화
        let json = serde_json::to_string(&price).unwrap();
        
        // Then
        assert_eq!(json, r#"{"satoshis":7000012345678}"#);

        // When - JSON 역직렬화
        let deserialized: SafeBtcPrice = serde_json::from_str(&json).unwrap();
        
        // Then
        assert_eq!(deserialized, price);
    }

    #[test]
    fn test_safe_price_extreme_values() {
        // Given - 최대 BTC 공급량 (21M BTC)
        let max_supply = SafeBtcPrice::from_price(21_000_000.0);
        assert_eq!(max_supply.as_satoshis(), 2_100_000_000_000_000);

        // Given - 최소 단위 (1 satoshi)
        let min_unit = SafeBtcPrice::from_satoshis(1);
        assert_eq!(min_unit.as_price(), 0.00000001);

        // Given - 0
        let zero = SafeBtcPrice::from_price(0.0);
        assert_eq!(zero.as_satoshis(), 0);
    }
}