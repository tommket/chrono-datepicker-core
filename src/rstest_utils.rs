use chrono::NaiveDate;
use rstest::fixture;

use crate::{viewed_date::{DayNumber, MonthNumber, YearNumber}, utils::from_ymd};

#[fixture(year = 1990, month = 1, day = 1)]
pub fn create_date(year: YearNumber, month: MonthNumber, day: DayNumber) -> NaiveDate {
    from_ymd(year, month, day)
}
