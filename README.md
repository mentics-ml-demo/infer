# infer

Inference component. Streams input from redpanda and runs against current model.

Inference can be rather slow. To horizontally scale this, the following in the main thread:

1) Combine the processing time metrics into a single metric. It will be initialized to some conservative reasonable value.
2) Periodically update the time slots on the worker threads based on the processing time metric.
3) Maintain an events/second metric (probably EMA) on how many events occur per time slot and report this to the worker threads.
4) It also handles model updating and telling the worker threads when to switch between models.

Each thread that runs processing will target different data by doing the following:

1) Maintain a processing time metric (maybe p95 or similar) for how long inference + persistence takes and report that to the main thread
2)
