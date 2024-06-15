mod inferer;

use std::{collections::{HashMap, VecDeque}, env};

use convert::series_to_input;
use model::{inputs_to_device, outputs_to_device};
use shared_types::*;
use data_info::*;
use series_proc::{BaseHandler, Processor};
use series_store::*;
use kv_store::*;
use shared_burn::*;
use quote::{QuoteEvent, QuoteValues};
use inferer::{make_inferer, Inferer};

// TODO: run metrics on true/false positive/negatives for each output element

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut run_count = 10;
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
            run_count = arg_count;
        }
    }

    let topic = Topic::new("raw", "SPY", "quote");
    let series = SeriesReader::new_topic("infer", &topic)?;
    let store = KVStore::new(CURRENT_VERSION).await?;

    let start_offset = store.next_safe_predict_offset().await?;
    series.seek(start_offset - SERIES1_LENGTH + 1)?;

    let infer = Infer::new(store, start_offset, run_count).await?;
    let mut handler: BaseHandler<QuoteValues, QuoteEvent, _> = BaseHandler::new(infer);
    series.for_each_msg(&mut handler).await;

    handler.proc.output_results();

    Ok(())
}

struct Infer {
    inferer: Inferer<TheBackend>,
    store: KVStore,
    count: usize,
    run_count: usize,
    lookup: HashMap<EventId,LabelType>,
}

impl Infer {
    async fn new(store: KVStore, start_offset: OffsetId, run_count: usize) -> anyhow::Result<Self> {
        let infer = make_inferer()?;
        let lookup = Self::load_lookup(&store, start_offset, run_count).await?;
        Ok(Infer { inferer: infer, store, count: 0, run_count, lookup })
    }

    async fn load_lookup(store: &KVStore, start_offset: OffsetId, count: usize) -> anyhow::Result<HashMap<EventId,LabelType>> {
        let rows = store.label_lookup(start_offset, count).await?;
        let mut map = HashMap::new();
        for row in rows {
            println!("Added lookup for offset {}", row.offset_from);
            map.insert(row.event_id, row.label);
        }
        println!("Loaded {} entries into lookup map", map.len());
        println!("keys: {:?}", map.keys());
        Ok(map)
    }

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

        let input_raw = match series_to_input(events) {
            Ok(input) => input,
            Err(e) => {
                println!("Failed to convert series to input for event_id {}. Aborting. {:?}", event_id, e);
                return false;
            },
        };
        let input = inputs_to_device(&vec![input_raw], &self.inferer.device);

        match self.lookup.get(&event_id) {
            Some(label_raw) => {
                let expected = outputs_to_device(vec![*label_raw], &self.inferer.device);
                self.inferer.infer_and_check(input, expected, true);
            },
            None => {
                let output = self.inferer.infer_batch(input);
                println!("No lookup found for event_id {}, output: {:?}", event_id, output);
                    // Ok(output) => {
                    //     println!("No lookup found for event_id {}, output: {:?}", event_id, output);
                    // },
                    // Err(e) => {
                    //     println!("Inference failed for event_id {}. Aborting. {:?}", event_id, e);
                    //     return false;
                    // }
            }
        }

        self.count += 1;
        if self.count >= self.run_count {
            println!("TODO: debug stopping at event_id: {}", event_id);
            false
        } else {
            true
        }
    }

    fn output_results(&self) {
        println!("Results: {:#?}", self.inferer.metrics);
        self.inferer.metrics.print_summary();
    }
}

impl Processor<VecDeque<QuoteEvent>,QuoteValues> for Infer {
    fn process(&mut self, _start_values: &QuoteValues, events: &mut VecDeque<QuoteEvent>) -> bool {
        if events.len() == SERIES1_SIZE {
            let first = &events[0];
            let last = &events[SERIES1_SIZE-1];
            println!("Found series from offsets {} to {}, event_id: {}", first.offset, last.offset, last.event_id);
            let result = self.handle_series(events);
            let _ = events.pop_front();
            result
        } else {
            true
        }
    }
}
