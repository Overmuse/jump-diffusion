use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use datastore_client::{Client as DatastoreClient, GetLastOpen};
use futures::prelude::*;
use itertools::Itertools;
use polygon::rest::{Client as PolygonClient, GetAggregate, SortOrder, Timespan};
use rust_decimal::prelude::*;
use tracing::debug;

use crate::aggregates::JumpDiffusionAggregate;

#[derive(Debug, Clone)]
pub struct Data {
    pub ticker: String,
    pub log_returns: Vec<f64>,
    pub current_price: Decimal,
}

async fn download_ticker_data(
    polygon_client: &PolygonClient<'_>,
    datastore_client: &DatastoreClient<'_>,
    ticker: &str,
    date: &NaiveDate,
) -> Result<Data> {
    let agg = polygon_client
        .send(
            GetAggregate::new(ticker, *date, *date)
                .multiplier(1)
                .timespan(Timespan::Minute)
                .unadjusted(false)
                .sort(SortOrder::Asc),
        )
        .await?;
    let open = datastore_client
        .send(GetLastOpen::new(ticker.to_string()))
        .await?;
    if let Some(res) = agg.results {
        if open.is_none() {
            return Err(anyhow!("Missing open price for ticker {}", ticker));
        };
        let open = Decimal::from_f64(open.expect("Guaranteed to exist")).unwrap();
        let prices = res
            .iter()
            .filter(|x| x.is_open())
            .map(|x| x.c)
            .chain(std::iter::once(open));
        let log_returns = prices
            .tuple_windows()
            .map(|(p1, p2)| (p2.ln() - p1.ln()).to_f64().unwrap())
            .collect();
        Ok(Data {
            ticker: ticker.to_string(),
            log_returns,
            current_price: open,
        })
    } else {
        Err(anyhow!("Missing data"))
    }
}

pub async fn download_data(
    polygon_client: &PolygonClient<'_>,
    datastore_client: &DatastoreClient<'_>,
    tickers: &[String],
    date: NaiveDate,
) -> Vec<Result<Data>> {
    debug!("Beginning data download");
    stream::iter(tickers)
        .map(|ticker| download_ticker_data(polygon_client, datastore_client, ticker, &date))
        .buffered(100)
        .collect()
        .await
}
