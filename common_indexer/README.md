# azero-tools-rs contract indexer

## Running instructions

You need cargo installed to run this project. To run the webserver 
```
cd common_indexer
cargo run --release
``` 
This will run the server at `http://127.0.0.1:3001`.

## Example queries

```
curl "http://localhost:3001/trades?block_start=84122149&block_stop=84132249"
```


```
curl "http://localhost:3001/status"
```



