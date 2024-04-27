use series_store::{SeriesReader, SeriesWriter};
use kv_store::*;
use shared_types::{Event, Inference, Inferred, Logger, StdoutLogger};

// TODO: When it first comes up, check if there are enough messages in series-store in the past to fill the buffer.

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let reader = SeriesReader::new(Box::new(StdoutLogger()))?;
    let store = KVStore::new().await?;
    let logger = StdoutLogger();

    let predict = Predict::new(reader, store, Box::new(logger));

    predict.run().await;

    // println!("Adding key-values");
    // let kv1 = KeyValue { id: 1, time: 100, datum1: 11.01, datum2: 12.0 };
    // let kv2 = KeyValue { id: 2, time: 103, datum1: 12.20, datum2: 4.2323 };
    // let kv3 = KeyValue { id: 3, time: 107, datum1: 9.50, datum2: 3.000001111 };
    // add_key_value(&session, kv1).await?;
    // add_key_value(&session, kv2).await?;
    // add_key_value(&session, kv3).await?;

    // println!("Selecting key-values");
    // let kvs = select_key_value(&session, 2).await?;
    // println!("Found {:?}", kvs);
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

        // self.reader.foreach_event(|event| {
        //     let res = self.on_event(event).await;
        // });
        Ok(())
    }

    async fn on_event(&self, event: Event) {
        println!("Processing event {:?}", event);
        // TODO: buffer
        let inf = Inferred { id: event.id, timestamp: shared_types::now(), inference: Inference::default() };
        self.store.write_inference(&inf).await.map_err(|e| {
            self.logger.log(format!("Error inserting inferred {:?}", &inf));
        });
    }
}
