use anyhow::Result;
use chrono::NaiveDate;
use itertools::Itertools;
use polygon::rest::{Client, GetAggregate, GetTickerSnapshot, SortOrder, Timespan};
use rust_decimal::prelude::*;

use crate::aggregates::JumpDiffusionAggregate;

#[derive(Debug, Clone)]
pub struct Data {
    pub log_returns: Vec<f64>,
    pub current_price: f64,
}

pub async fn download_data(
    client: &Client<'_>,
    tickers: &[String],
    date: NaiveDate,
) -> Result<Vec<Data>> {
    let past_day = tickers.iter().map(|ticker| {
        GetAggregate::new(&ticker, date, date)
            .multiplier(1)
            .timespan(Timespan::Minute)
            .unadjusted(false)
            .sort(SortOrder::Asc)
    });
    let past_day_results = client.send_all(past_day).await;
    let snapshots = tickers.iter().map(|ticker| GetTickerSnapshot(ticker));
    let snapshot_results = client.send_all(snapshots).await;
    let log_returns: Vec<_> = past_day_results
        .into_iter()
        .zip(snapshot_results.into_iter())
        .filter_map(|(past, snap)| match (past, snap) {
            (Ok(p), Ok(s)) => Some((p, s)),
            _ => None,
        })
        .filter_map(|(past, snap)| past.results.map(|res| (res, snap.ticker.day)))
        .map(|(yesterday, today)| {
            let prices = yesterday
                .iter()
                .filter(|x| x.is_open())
                .map(|x| x.c.to_f64().unwrap())
                .chain(std::iter::once(today.o.to_f64().unwrap()));
            let log_returns: Vec<f64> = prices
                .tuple_windows::<(_, _)>()
                .map(|(p1, p2)| (f64::ln(p2) - f64::ln(p1)))
                .collect();
            Data {
                log_returns,
                current_price: today.c.to_f64().unwrap(),
            }
        })
        .collect();
    Ok(log_returns)
}
