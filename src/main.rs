use anyhow::{Context, Result};
use chrono::{Duration, Local, NaiveTime, TimeZone, Utc};
use chrono_tz::US::Eastern;
use datastore_client::Client as DatastoreClient;
use kafka_settings::producer;
use polygon::rest::{Client as PolygonClient, GetPreviousClose};
use rdkafka::producer::FutureRecord;
use rust_decimal::prelude::*;
use tracing::{debug, error, info, subscriber::set_global_default};
use tracing_subscriber::EnvFilter;
use trading_base::{Amount, PositionIntent};

mod aggregates;
mod data;
mod jumps;
mod math;
mod settings;

use data::{download_data, Data};
use jumps::{find_jump, jump_detected};
use math::zscore;
use settings::Settings;

#[derive(Debug, Clone)]
struct Evaluation {
    ticker: String,
    price: Decimal,
    z_score: f64,
    last_ret: f64,
}

fn choose_stocks(data: &[Data], n: usize) -> Vec<Evaluation> {
    let mut zscores: Vec<Evaluation> = data
        .iter()
        .filter_map(|data| {
            let col = data.log_returns.as_slice();
            let len = col.len();
            let (idx, _) = find_jump(col);
            let z = zscore(col);
            if jump_detected(col) && idx == (len - 1) {
                Some(Evaluation {
                    ticker: data.ticker.clone(),
                    price: data.current_price,
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
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    set_global_default(subscriber).expect("Failed to set subscriber");
    info!("Starting jump-diffusion");
    let settings = Settings::new().context("Failed to load settings")?;
    let tickers = settings.app.tickers;
    let cash = settings.app.initial_equity;
    let producer = producer(&settings.kafka).context("Failed to initialize Kafka producer")?;
    let datastore_client = DatastoreClient::from_env()
        .context("Failed to create datastore client from environment variables")?;
    let polygon_client = PolygonClient::from_env()
        .context("Failed to create polygon client from environment variables")?;
    // Use `GetPreviousClose` in order to find the previous close *date*
    debug!("Fetching previous close date");
    let res = polygon_client
        .send(GetPreviousClose {
            ticker: &tickers[0],
            unadjusted: false,
        })
        .await
        .context("Failed to get previous close")?;

    let last_trading_date = res.results[0].t.naive_utc().date();
    debug!("Downloading data");
    let data = download_data(
        &polygon_client,
        &datastore_client,
        &tickers,
        last_trading_date,
    )
    .await;
    let data: Vec<Data> = data
        .into_iter()
        .filter_map(|x| match x {
            Ok(x) => Some(x),
            Err(e) => {
                error!("{}", e);
                None
            }
        })
        .collect();
    let stocks = choose_stocks(&data, settings.app.num_stocks);
    let sum_z: f64 = stocks.iter().map(|x| x.z_score.abs()).sum();
    for stock in stocks {
        let (dollars, limit_price) = if stock.last_ret.is_sign_positive() {
            (
                -(cash * Decimal::from_f64(stock.z_score.abs() / sum_z).unwrap()),
                stock.price * Decimal::new(995, 3),
            )
        } else {
            (
                (cash * Decimal::from_f64(stock.z_score.abs() / sum_z).unwrap()),
                stock.price * Decimal::new(1005, 3),
            )
        };
        debug!(
            ticker = %stock.ticker,
            z_score = %stock.z_score,
            price = %stock.price,
            last_return = %stock.last_ret,
            %dollars,
            %limit_price,
            "Evaluation"
        );
        let intent = PositionIntent::builder(
            "jump-diffusion",
            stock.ticker.clone(),
            Amount::Dollars(dollars),
        )
        .before(Utc::now() + Duration::minutes(30))
        .limit_price(limit_price)
        .decision_price(stock.price)
        .build()?;
        let close_time = Eastern
            .from_local_date(&Local::today().naive_local())
            .and_time(NaiveTime::from_hms(11, 30, 0))
            .unwrap()
            .with_timezone(&Utc);
        let close_intent =
            PositionIntent::builder("jump-diffusion", stock.ticker.clone(), Amount::Zero)
                .after(close_time)
                .build()?;
        for i in vec![intent, close_intent] {
            let payload = serde_json::to_string(&i)?;
            let record = FutureRecord::to("position-intents")
                .key(&stock.ticker)
                .payload(&payload);
            let res = producer
                .send(record, std::time::Duration::from_secs(0))
                .await;
            if let Err((e, m)) = res {
                error!(
                    "Failed to send position intent to kafka.\nError: {:?}\nMessage: {:?}",
                    e, m
                );
            }
        }
    }
    info!("All done");
    Ok(())
}
