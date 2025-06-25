# Oracle Node (PoC)

바이낸스 현물 API에서 BTC/USDT 가격을 수집하는 간단한 Oracle Node입니다.

## 기능

- 바이낸스 현물 API에서 BTC/USDT 가격 수집 (1분마다)
- 최근 100개 가격 데이터 메모리 저장
- HTTP API로 가격 데이터 제공

## 실행 방법

```bash
# Oracle Node 실행
cargo run --bin oracle-node

# 로그 확인
[2024-01-01T12:00:00Z INFO  oracle_node] Starting Oracle Node (PoC)...
[2024-01-01T12:00:00Z INFO  oracle_node::server] HTTP server listening on 127.0.0.1:8080
[2024-01-01T12:01:00Z INFO  oracle_node] Fetched BTC price: $43521.50
```

## API 엔드포인트

### 1. Health Check
```bash
GET http://localhost:8080/health
Response: "OK"
```

### 2. 현재 가격 조회
```bash
GET http://localhost:8080/price

Response:
{
  "price": 43521.50,
  "timestamp": 1704110460,
  "source": "binance"
}
```

### 3. 전체 가격 목록 조회
```bash
GET http://localhost:8080/prices

Response:
{
  "prices": [
    {
      "price": 43521.50,
      "timestamp": 1704110460,
      "source": "binance"
    },
    ...
  ],
  "count": 5
}
```

## 테스트

```bash
# 테스트 스크립트 실행
./scripts/test_oracle.sh
```

## 주의사항

- 이 코드는 PoC 수준이며, 프로덕션 사용을 위해서는 추가적인 보안 및 안정성 개선이 필요합니다
- 바이낸스 API rate limit에 주의하세요 (현재 1분마다 요청)
- 실제 운영시에는 API 키 인증이 필요할 수 있습니다