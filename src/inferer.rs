use burn::{nn::loss::{MseLoss, Reduction}, tensor::{backend::Backend, Tensor}};

use data_info::MODEL_OUTPUT_WIDTH;
use shared_burn::{burn_device, model::{ModelInput, ModelOutput, TheModel}, model_persist::load_model, output::print_compare_table, tensor1_to_vec, Model, TheBackend};
use shared_types::*;

pub fn make_inferer() -> anyhow::Result<Inferer<TheBackend>> {
    let device = burn_device();
    Inferer::new(device)
}

pub struct Inferer<B:Backend> {
    pub device: B::Device,
    model: TheModel<B>,
    loss_calc: MseLoss<B>,
    pub metrics: OutputMetrics
}

impl<B:Backend> Inferer<B> {
    pub fn new(device: B::Device) -> anyhow::Result<Self> {
        let model = load_model(&device)?;
        let loss = MseLoss::new();

        println!("Is autodiff enabled for infer backend: {}", B::ad_enabled());
        assert!(!B::ad_enabled());

        Ok(Self { device, model, loss_calc: loss, metrics: OutputMetrics::default() })
    }

    // pub fn infer_and_check(&mut self, input1: ModelInput1<B>, expected: ModelOutput1<B>, display_result: bool) {
    //     let input = (input1.0.unsqueeze(), input1.1.unsqueeze());
    //     let output = self.infer_batch(input).squeeze(0);
    //     // let expected = expected.to_tensor(&self.device);
    //     let losses = self.loss_calc.forward(output.clone(), expected.clone(), Reduction::Mean);

    //     if display_result {
    //         print_compare_table(output.clone().unsqueeze(), expected.clone().unsqueeze(), &losses);
    //     }

    //     self.metrics.add(output, expected);

    //     // let v = losses.into_data().convert::<f32>().value;
    //     // assert!(v.len() == 1);

    //     // v[0]
    // }

    pub fn infer_and_check(&mut self, input: ModelInput<B>, expected: ModelOutput<B>, display_result: bool) {
        let output = self.infer_batch(input);
        let losses = self.loss_calc.forward(output.clone(), expected.clone(), Reduction::Mean);

        if display_result {
            // print_compare_table(output.clone().unsqueeze(), expected.clone().unsqueeze(), &losses);
            print_compare_table(output.clone(), expected.clone(), &losses);
        }

        self.metrics.add(output.squeeze(0), expected.squeeze(0));

        // let v = losses.into_data().convert::<f32>().value;
        // assert!(v.len() == 1);

        // v[0]
    }

    // pub fn infer_1(&self, input1: ModelInput1<B>) -> ModelOutput1<B> {
    //     let input = (input1.0.unsqueeze(), input1.1.unsqueeze());
    //     let output = self.infer_batch(input).squeeze(0);
    //     output
    //     // output.into_data().convert::<f32>().value.try_into().map_err(|e| anyhow!("Error converting inference output to ModelOutput type {:?}", e))
    // }

    pub fn infer_batch(&self, input: ModelInput<B>) -> ModelOutput<B> {
        self.model.forward(input)
    }
}

#[derive(Default, Debug)]
pub struct OutputMetrics {
    count: u64,
    metrics: [Metrics; MODEL_OUTPUT_WIDTH]
}

impl OutputMetrics {
    fn add<B:Backend>(&mut self, output: Tensor<B,1>, expected: Tensor<B,1>) {
        let out = tensor1_to_vec(output);
        let exp = tensor1_to_vec(expected);
        for i in 0..MODEL_OUTPUT_WIDTH {
            self.metrics[i].add(out[i], exp[i]);
        }
        self.count += 1;
    }

    pub fn print_summary(&self) {
        let mut true_pos = 0;
        let mut false_pos = 0;
        for i in 0..MODEL_OUTPUT_WIDTH {
            let m = &self.metrics[i];
            true_pos += m.true_positives;
            false_pos += m.false_positives;
        }
        println!("true_pos / count: {} / {} : {}", true_pos, self.count, true_pos as f32 / self.count as f32);
        println!("false_pos / count: {} / {} : {}", false_pos, self.count, false_pos as f32 / self.count as f32);
    }
}

#[derive(Default, Debug)]
pub struct Metrics {
    true_positives: u64,
    true_negatives: u64,
    false_positives: u64,
    false_negatives: u64,
}

impl Metrics {
    fn add(&mut self, output: ModelFloat, actual: ModelFloat) {
        let positive = output >= 0.5;
        let actual_positive = actual >= 0.5;
        if positive && actual_positive {
            self.true_positives += 1;
        } else if positive && !actual_positive {
            self.false_positives += 1;
        } else if !positive && actual_positive {
            self.false_negatives += 1;
        } else if !positive && !actual_positive {
            self.true_negatives += 1;
        }
    }
}
