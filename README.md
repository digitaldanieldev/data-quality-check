# Data-quality-server

todo:
use criterion for benchmarking

## Enable metrics
./data-quality-server --enable-metrics
./data-quality-server --worker-threads 4 --enable-metrics


# Config-producer-proto

## Run Once (One-time processing)

This will process the .proto files and send them to the server once.

./config-producer-proto

## Run the program in a loop.

Check for .proto file updates every 30 seconds.
./config-producer-proto --loop --interval 30


## check if message is valid json
./data-quality-server --json '{"key1": "value1", "key2": 42}'

## enable / disable logging / set level as user
## set metrics collection interval

## Check is a message is valid JSON - test success no protobuf
curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}"  }'

## ## Check if JSON can be serialized using MyMessage protobuf definition - test success protobuf
curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "protobuf": "MyMessage", 
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}" }'

## Check if JSON can be serialized using MyMessage protobuf definition and validate that field key2 contains the number 42 - test success protobuf with added field validation
curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "protobuf": "MyMessage", 
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}", 
    "json_escaped": true,
    "field_check": true,
    "field_name": "key2",
    "field_value_check": 42  }'

curl -X POST http://localhost:8080/validate   -H "Content-Type: application/json"   -d '{
    "protobuf": "MyMessage", 
    "json": {"key1": "example_value", "key2": 42, "key3": true}, 
    "json_escaped": false,
    "field_check": true,
    "field_name": "key2",
    "field_value_check": 42
  }'

curl -X POST http://localhost:8080/validate \
    -H "Content-Type: application/json" \
    -d '{
        "protobuf": "MyMessage",
        "json": '"$(cat data.json)"',
        "json_escaped": false,
        "field_check": true,
        "field_name": "key2",
        "field_value_check": 42
    }'

with data.json:
{
    "key1": "example_value",
    "key2": 42,
    "key3": true
}

## wget
wget --quiet \
     --method=POST \
     --header="Content-Type: application/json" \
     --body-data='{
         "protobuf": "MyMessage",
         "json": {
             "key1": "example_value",
             "key2": 42,
             "key3": true
         },
         "json_escaped": false,
         "field_check": true,
         "field_name": "key2",
         "field_value_check": 42
     }' \
     -O - http://localhost:8080/validate

## httpie
http POST http://localhost:8080/validate \
    Content-Type:application/json \
    protobuf="MyMessage" \
    json:='{
        "key1": "example_value",
        "key2": 42,
        "key3": true
    }' \
    json_escaped:=false \
    field_check:=true \
    field_name="key2" \
    field_value_che
    
# tests

## bash
./load-test.sh --curl
SERVER_IP=192.168.178.106 SERVER_PORT=8080 ./load-test.sh --curl
./load-test.sh --wget
SERVER_IP=192.168.178.106 SERVER_PORT=8080 ./load-test.sh --wget

## docker-compose
docker-compose up load-test-curl


docker-compose up load-test-wget
docker-compose logs load-test-wget > wget_logs.txt
./requests-per-second.sh

docker-compose up load-test-wget --scale load-test-wget=5

load-test-wget_logs.txt
