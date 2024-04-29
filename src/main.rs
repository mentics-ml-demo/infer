use series_store::SeriesReader;
use kv_store::*;
use shared_types::{Event, Inference, Inferred, Logger, StdoutLogger};

// TODO: When it first comes up, check if there are enough messages in series-store in the past to fill the buffer.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let reader = SeriesReader::new(Box::new(StdoutLogger()))?;
    let store = KVStore::new().await?;
    let logger = StdoutLogger();

    let predict = Predict::new(reader, store, Box::new(logger));

    predict.run().await?;

    Ok(())
}

struct Predict {
    reader: SeriesReader,
    store: KVStore,
    logger: Box<dyn Logger>,
}

impl Predict {
    fn new(reader: SeriesReader, store: KVStore, logger: Box<dyn Logger>) -> Self {
        Predict { reader, store, logger }
    }

    async fn run(&self) -> anyhow::Result<()> {
        self.reader.seek(&self.reader.topics.event, -100)?;
        let future = move |event| async move {
            self.on_event(event).await;
        };
        self.reader.foreach_event(future).await;

        Ok(())
    }

    async fn on_event(&self, event: Event) {
        println!("Processing event {:?}", event);
        // TODO: buffer
        let inf = Inferred { id: event.id, timestamp: shared_types::now(), inference: Inference::default() };
        let _ = self.store.write_inference(&inf).await.map_err(|e| {
            self.logger.log(format!("Error {} inserting inferred {:?}", e, &inf));
        });
    }
}
