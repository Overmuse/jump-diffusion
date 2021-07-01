use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use futures::prelude::*;
use itertools::Itertools;
use polygon::rest::{Client, GetAggregate, GetTickerSnapshot, SortOrder, Timespan};
use rust_decimal::prelude::*;
use tracing::debug;

use crate::aggregates::JumpDiffusionAggregate;

#[derive(Debug, Clone)]
pub struct Data {
    pub ticker: String,
    pub log_returns: Vec<f64>,
    pub current_price: Decimal,
}

async fn download_ticker_data(client: &Client<'_>, ticker: &str, date: &NaiveDate) -> Result<Data> {
    let agg = client
        .send(
            GetAggregate::new(ticker, *date, *date)
                .multiplier(1)
                .timespan(Timespan::Minute)
                .unadjusted(false)
                .sort(SortOrder::Asc),
        )
        .await?;
    let snapshot = client.send(GetTickerSnapshot(ticker)).await?;
    if let Some(res) = agg.results {
        let today = snapshot.ticker.day;
        let prices = res
            .iter()
            .filter(|x| x.is_open())
            .map(|x| x.c)
            .chain(std::iter::once(today.o));
        let log_returns = prices
            .tuple_windows()
            .map(|(p1, p2)| (p2.ln() - p1.ln()).to_f64().unwrap())
            .collect();
        Ok(Data {
            ticker: ticker.to_string(),
            log_returns,
            current_price: today.c,
        })
    } else {
        Err(anyhow!("Missing data"))
    }
}

pub async fn download_data(
    client: &Client<'_>,
    tickers: &[String],
    date: NaiveDate,
) -> Vec<Result<Data>> {
    debug!("Beginning data download");
    stream::iter(tickers)
        .map(|ticker| download_ticker_data(client, ticker, &date))
        .buffered(100)
        .collect()
        .await
}
