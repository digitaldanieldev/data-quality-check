
curl -X POST http://localhost:8080/validate \
     -H "Content-Type: application/json" \
     -d '{"n": "MyMessage", "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}"}'