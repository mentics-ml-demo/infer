use std::{collections::{HashMap, VecDeque}, env};

use shared_types::*;
use series_proc::BaseHandler;
use series_store::*;
use kv_store::*;
use train::{convert::*, inferer::{make_inferer, Inferer}, TheBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut count = 10;
    // let mut reset = false;
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut index = 1;
        let mut arg = &args[index];
        if arg == "reset" {
            // reset = true;
            index += 1;
        }
        arg = &args[index];
        if let Ok(arg_count) = arg.parse::<usize>() {
            count = arg_count;
        }
    }

    let topic = Topic::new("raw", "SPY", "quote");
    let series = SeriesReader::new_topic(StdoutLogger::boxed(), "infer", &topic)?;
    let store = KVStore::new(CURRENT_VERSION).await?;

    let offset = store.next_safe_predict_offset().await?;
    series.seek(offset - SERIES_LENGTH + 1)?;

    let predict = Infer::new(store, count, StdoutLogger::boxed()).await?;
    let mut handler: BaseHandler<QuoteValues, QuoteEvent, _> = BaseHandler::new(predict);
    series.for_each_msg(&mut handler).await;

    // predict.run().await?;

    Ok(())
}

struct Infer {
    logger: Box<dyn Logger>,
    inferer: Inferer<TheBackend>,
    store: KVStore,
    count: usize,
    max: usize,
    lookup: HashMap<EventId,LabelType>,
}

impl Infer {
    async fn new(store: KVStore, max: usize, logger: Box<dyn Logger>) -> anyhow::Result<Self> {
        let infer = make_inferer()?;
        let lookup = Self::load_lookup(&store, max).await?;
        Ok(Infer { logger, inferer: infer, store, count: 0, max, lookup })
    }

    async fn load_lookup(store: &KVStore, count: usize) -> anyhow::Result<HashMap<EventId,LabelType>> {
        let rows = store.label_lookup(START_OFFSET_ID, count).await?;
        // select event_id, label from ml_demo.labeled where version=1 and offset_from >= 1089327 order by event_id ASC limit 10 allow filtering;
        // , offset_from, offset_to
        let mut map = HashMap::new();
        for row in rows {
            println!("Added lookup for offset {}", row.offset_from);
            map.insert(row.event_id, row.label);
        }
        println!("Loaded {} entries into lookup map", map.len());
        println!("keys: {:?}", map.keys());
        Ok(map)
    }

    // async fn run(&self) -> anyhow::Result<()> {

        // let handler = BaseHandler::new(self);
        // let mut test = Test {};
        // self.series.for_each_msg(&mut test).await;

            // if !self.handler.is_done() {
            //     // I think this can never happen. Maybe put in type system?
            //     println!("This shouldn't happen. Ran out of messages before labelling? Aborting.");
            //     return Ok(())
            // }

            // println!("Ended at {:?}", self.handler.ids());
            // self.store_result().await?;

    //     Ok(())
    // }

    // async fn on_event(&self, event: Event) {
    //     println!("Processing event {:?}", event);
    //     // TODO: buffer
    //     let inf = Inferred { id: event.id, timestamp: shared_types::now(), inference: Inference::default() };
    //     let _ = self.store.write_inference(&inf).await.map_err(|e| {
    //         self.logger.log(format!("Error {} inserting inferred {:?}", e, &inf));
    //     });
    // }

    async fn store_result(&self) -> anyhow::Result<()> {
        // let timestamp = now();
        // let LabelIds { event_id, offset_from, offset_to } = self.handler.ids();
        // let labeled = LabelStored {
        //     event_id,
        //     timestamp,
        //     partition: PARTITION,
        //     offset_from,
        //     offset_to,
        //     label: Self::make_label(&self.handler.complete)
        // };
        // self.store.label_store(&labeled).await?;
        // let event = LabelEvent::new(event_id, timestamp, offset_from, offset_to, labeled.label);
        // let json = serde_json::to_string(&event)?;
        // println!("Writing event_id: {event_id}, offset_from: {offset_from} to label series {}", self.label_topic.name);
        // self.series_label.write(event_id, &self.label_topic, "key", timestamp, &json).await?;
        Ok(())
    }

    fn handle_series(&mut self, events: &mut VecDeque<QuoteEvent>) -> bool {
        let event_id = events.back().unwrap().event_id;

        let input = match series_to_input(events) {
            Ok(input) => input,
            Err(e) => {
                println!("Failed to convert series to input for event_id {}. Aborting. {:?}", event_id, e);
                return false;
            },
        };

        match self.lookup.get(&event_id) {
            Some(label) => {
                let _ = self.inferer.infer_and_check(input, *label, true);
            },
            None => {
                match self.inferer.infer_1(input) {
                    Ok(output) => {
                        println!("No lookup found for event_id {}, output: {:?}", event_id, output);
                    },
                    Err(e) => {
                        println!("Inference failed for event_id {}. Aborting. {:?}", event_id, e);
                        return false;
                    }
                }
            }
        };

        self.count += 1;
        if self.count >= self.max {
            println!("TODO: debug stopping at event_id: {}", event_id);
            false
        } else {
            true
        }
    }
}

impl Processor<VecDeque<QuoteEvent>,QuoteValues> for Infer {
    fn process(&mut self, _start_values: &QuoteValues, events: &mut VecDeque<QuoteEvent>) -> bool {
        if events.len() == SERIES_SIZE {
            let first = &events[0];
            let last = &events[SERIES_SIZE-1];
            println!("Found series from offsets {} to {}, event_id: {}", first.offset, last.offset, last.event_id);
            let result = self.handle_series(events);
            let _ = events.pop_front();
            result
        } else {
            true
        }
    }
}
