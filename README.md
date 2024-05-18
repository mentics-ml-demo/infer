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


------------

For creating a container to run lag-llama
docker run --name lagl -d ubuntu:latest /bin/bash -c "sleep infinity"
docker exec -it lagl /bin/bash
useradd lagl
chsh -s /bin/bash lagl
apt-get install -y python3 python3-venv git vim
su - lagl
mkdir python
cd python
python3 -m venv "venv"
source venv/bin/activate
git clone https://github.com/time-series-foundation-models/lag-llama/
cd lag-llama
pip install -r requirements.txt
# the pip install command installs the huggingface cli

cd ..
mkdir infer
cd infer
huggingface-cli download time-series-foundation-models/Lag-Llama lag-llama.ckpt --local-dir .
# ${HOME}/python/lag-llama

python3 -i lag-llama.py

# Test:

dataset = get_dataset("m4_weekly")

prediction_length = dataset.metadata.prediction_length
context_length = prediction_length*3
num_samples = 20
device = "cuda"

forecasts, tss = get_lag_llama_predictions(
    dataset.test,
    prediction_length=prediction_length,
    num_samples=num_samples,
    context_length=context_length,
    device=device
)



---------- collab 1 one-host demo -------------

import pandas as pd
from gluonts.dataset.pandas import PandasDataset

url = (
    "https://gist.githubusercontent.com/rsnirwan/a8b424085c9f44ef2598da74ce43e7a3"
    "/raw/b6fdef21fe1f654787fa0493846c546b7f9c4df2/ts_long.csv"
)
df = pd.read_csv(url, index_col=0, parse_dates=True)
df

# Set numerical columns as float32
for col in df.columns:
    # Check if column is not of string type
    if df[col].dtype != 'object' and pd.api.types.is_string_dtype(df[col]) == False:
        df[col] = df[col].astype('float32')

# Create the Pandas
dataset = PandasDataset.from_long_dataframe(df, target="target", item_id="item_id")

backtest_dataset = dataset
prediction_length = 24  # Define your prediction length. We use 24 here since the data is of hourly frequency
num_samples = 100 # number of samples sampled from the probability distribution for each timestep
device = torch.device("cuda:0")

forecasts, tss = get_lag_llama_predictions(backtest_dataset, prediction_length, device, num_samples)


