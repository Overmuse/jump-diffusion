use chrono::prelude::*;
use chrono_tz::America::New_York;
use polygon::rest::Aggregate;

pub trait JumpDiffusionAggregate {
    fn is_open(&self) -> bool;
}

impl JumpDiffusionAggregate for Aggregate {
    fn is_open(&self) -> bool {
        let datetime = Utc.timestamp(self.t as i64 / 1000, 0);
        let zoned = datetime.with_timezone(&New_York);
        (zoned.time() >= NaiveTime::from_hms(9, 30, 00))
            && (zoned.time() < NaiveTime::from_hms(16, 00, 00))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal::prelude::*;

    #[test]
    fn is_open() {
        let agg = Aggregate {
            o: Decimal::new(100, 2),
            h: Decimal::new(100, 2),
            l: Decimal::new(100, 2),
            c: Decimal::new(100, 2),
            v: Decimal::new(100, 2),
            vw: Some(Decimal::new(100, 2)),
            t: 1620999000000,
            n: Some(100),
        };
        assert!(agg.is_open())
    }

    #[test]
    fn is_not_open() {
        let agg = Aggregate {
            o: Decimal::new(100, 2),
            h: Decimal::new(100, 2),
            l: Decimal::new(100, 2),
            c: Decimal::new(100, 2),
            v: Decimal::new(100, 2),
            vw: Some(Decimal::new(100, 2)),
            t: 1620998999999,
            n: Some(100),
        };
        assert!(!agg.is_open())
    }
}
