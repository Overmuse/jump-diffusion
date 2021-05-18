use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use kafka_settings::producer;
use polygon::rest::Client;
use rdkafka::producer::FutureRecord;

mod aggregates;
mod data;
mod jumps;
mod math;
mod positions;
mod settings;

use jumps::{find_jump, jump_detected};
use math::zscore;
use positions::PositionIntent;
use settings::Settings;

#[derive(Debug, Clone)]
struct Evaluation {
    idx: usize,
    z_score: f64,
    last_ret: f64,
}

fn choose_stocks(data: &[Vec<f64>], n: usize) -> Vec<Evaluation> {
    let len = data.first().unwrap().iter().count();
    let mut zscores: Vec<Evaluation> = data
        .iter()
        .enumerate()
        .filter_map(|(i, col)| {
            let (idx, _) = find_jump(&col);
            let z = zscore(&col);
            if jump_detected(&col) && idx == (len - 1) {
                Some(Evaluation {
                    idx: i,
                    z_score: z,
                    last_ret: *col.last().unwrap(),
                })
            } else {
                None
            }
        })
        .collect();
    zscores.sort_unstable_by(|x1, x2| x2.z_score.abs().partial_cmp(&x1.z_score.abs()).unwrap());
    zscores.into_iter().take(n).collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    let settings = Settings::new()?;
    let tickers = settings.app.tickers;
    let cash = settings.app.initial_equity;
    let producer = producer(&settings.kafka)?;
    let client =
        Client::from_env().context("Failed to create client from environment variables")?;
    let data = data::download_data(&client, &tickers, NaiveDate::from_ymd(2021, 5, 18))
        .await
        .context("Failed to download data")?;
    let (log_returns, current_prices): (Vec<_>, Vec<_>) = data
        .into_iter()
        .map(|x| (x.log_returns, x.current_price))
        .unzip();
    let stocks = choose_stocks(&log_returns, settings.app.num_stocks);
    let sum_z: f64 = stocks.iter().map(|x| x.z_score.abs()).sum();
    for stock in stocks {
        let qty = if stock.last_ret.is_sign_positive() {
            (cash * stock.z_score.abs() / sum_z) / current_prices[stock.idx]
        } else {
            -(cash * stock.z_score.abs() / sum_z) / current_prices[stock.idx]
        };
        let ticker = tickers[stock.idx].clone();
        let intent = PositionIntent {
            strategy: "jump-diffusion".into(),
            timestamp: Utc::now(),
            qty: qty.floor() as i32,
            ticker: ticker.clone(),
        };
        let payload = serde_json::to_string(&intent)?;
        let record = FutureRecord::to("position-intents")
            .key(&ticker)
            .payload(&payload);
        let res = producer
            .send(record, std::time::Duration::from_secs(0))
            .await;
        if let Err((e, _)) = res {
            panic!("{}", e)
        }
    }
    Ok(())
}
