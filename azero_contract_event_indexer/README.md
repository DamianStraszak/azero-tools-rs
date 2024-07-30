# azero-tools-rs contract indexer

## Running instructions

You need cargo installed to run this project. To run the webserver for psp22 tokens run
```
cd azero_contract_event_indexer
cargo run --release
``` 
This will run the server at `http://127.0.0.1:3000`.

## Example queries

```
curl "http://localhost:3000/events?block_start=84122189&block_stop=84122200" 
```


```
curl "http://localhost:3000/events_by_contract?block_start=84122149&block_stop=84122249&contract_address=5EWD7jTAf7ERr8wNA8JnaUG1tupoUx6VgoDHEGg5tis85s42"
```



