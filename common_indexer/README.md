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
curl "http://localhost:3000/events_by_range?block_start=84122149&block_stop=84122249"
```


```
curl "http://localhost:3000/events_by_contract?block_start=84122149&block_stop=84122249&contract_address=5EWD7jTAf7ERr8wNA8JnaUG1tupoUx6VgoDHEGg5tis85s42"
```



