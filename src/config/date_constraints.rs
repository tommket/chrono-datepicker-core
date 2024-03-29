use chrono::prelude::*;
use std::collections::HashSet;

use num_traits::FromPrimitive;

use crate::{
    utils::from_ymd,
    viewed_date::{year_group_range, ViewedDate},
};

#[cfg(test)]
use mockall::automock;

/// Trait that can be implemented to create your own date constraints.
#[cfg_attr(test, automock)]
pub trait HasDateConstraints {
    /// Returns true if the given date is forbidden.
    fn is_day_forbidden(&self, date: &NaiveDate) -> bool;

    /// Returns true if the entire month described by year_month_info is forbidden.
    fn is_month_forbidden(&self, year_month_info: &NaiveDate) -> bool;

    /// Returns true if the entire given year is forbidden.
    fn is_year_forbidden(&self, year: i32) -> bool;

    /// Returns true if the entire group of years including the given year is forbidden.
    /// A group of years are inclusive intervals [1980, 1999], [2000, 2019], [2020, 2039], ...
    fn is_year_group_forbidden(&self, year: i32) -> bool;
}

/// Date constraints configuration
#[derive(Default, Debug, Clone, Builder)]
#[builder(setter(strip_option))]
#[builder(default)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct DateConstraints {
    /// inclusive minimal date constraint
    /// the earliest date that can be selected
    min_date: Option<NaiveDate>,

    /// inclusive maximal date constraint
    /// the latest date that can be selected
    max_date: Option<NaiveDate>,

    /// disabled weekdays, that should not be selectable
    disabled_weekdays: HashSet<Weekday>,

    /// entire completely disabled months in every year
    disabled_months: HashSet<Month>,

    /// entire completely disabled years
    disabled_years: HashSet<i32>,

    /// disabled monthly periodically repeating dates, so it is just a day number
    /// starting from 1 for the first day of the month
    /// if unique dates in a certain year should not be selectable use `disabled_unique_dates`
    disabled_monthly_dates: HashSet<u32>,

    /// disabled yearly periodically repeating dates that should not be selectable,
    /// if unique dates in a certain year should not be selectable use `disabled_unique_dates`
    /// it is a `Vec` since we need to iterate over it anyway, since we hae no MonthDay type
    disabled_yearly_dates: Vec<NaiveDate>,

    /// disabled unique dates with a specific year, month and day that should not be selectable,
    /// if some periodically repeated dates should not be selectable use the correct option
    disabled_unique_dates: HashSet<NaiveDate>,
}

impl DateConstraintsBuilder {
    fn validate(&self) -> Result<(), String> {
        if let (Some(min_date), Some(max_date)) = (self.min_date, self.max_date) {
            if min_date > max_date {
                return Err("min_date must be earlier or exactly at max_date".into());
            }
        }
        Ok(())
    }
}

// TODO: find out how to place #[derive(Clone)] on the structure generated by automock
// this is a temporary workaround for tests
cfg_if::cfg_if! {
    if #[cfg(test)] {
        impl Clone for MockHasDateConstraints {
            fn clone(&self) -> Self {
                Self::new()
            }
        }
    }
}

impl HasDateConstraints for DateConstraints {
    fn is_day_forbidden(&self, date: &NaiveDate) -> bool {
        self.min_date.map_or(false, |min_date| &min_date > date)
            || self.max_date.map_or(false, |max_date| &max_date < date)
            || self.disabled_weekdays.contains(&date.weekday())
            || self
                .disabled_months
                .contains(&Month::from_u32(date.month()).unwrap())
            || self.disabled_years.contains(&date.year())
            || self.disabled_unique_dates.contains(date)
            || self.disabled_monthly_dates.contains(&date.day())
            || self
                .disabled_yearly_dates
                .iter()
                .any(|disabled| disabled.day() == date.day() && disabled.month() == date.month())
    }

    fn is_month_forbidden(&self, year_month_info: &NaiveDate) -> bool {
        self.disabled_years.contains(&year_month_info.year())
            || self
                .disabled_months
                .contains(&Month::from_u32(year_month_info.month()).unwrap())
            || year_month_info
                .first_day_of_month()
                .iter_days()
                .take_while(|date| date.month() == year_month_info.month())
                .all(|date| self.is_day_forbidden(&date))
    }

    fn is_year_forbidden(&self, year: i32) -> bool {
        self.disabled_years.contains(&year)
            || (1..=12u32).all(|month| self.is_month_forbidden(&from_ymd(year, month, 1)))
    }

    fn is_year_group_forbidden(&self, year: i32) -> bool {
        year_group_range(year).all(|year| self.is_year_forbidden(year))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rstest_utils::create_date,
        viewed_date::{DayNumber, MonthNumber, YearNumber},
    };
    use chrono::Duration;
    use rstest::*;

    #[rstest(
        tested_date, //
        case(create_date(1, 12, 25)),
        case(create_date(3000, 3, 22)),
    )]
    fn is_day_forbidden_default_no_bounds(tested_date: NaiveDate) {
        assert!(!DateConstraints::default().is_day_forbidden(&tested_date))
    }

    #[rstest(
        tested_date, //
        case(create_date(1, 12, 25)),
        case(create_date(3000, 3, 22)),
    )]
    fn is_month_forbidden_default_no_bounds(tested_date: NaiveDate) {
        assert!(!DateConstraints::default().is_month_forbidden(&tested_date))
    }

    #[rstest(
        tested_year, //
        case(1),
        case(3000),
    )]
    fn is_year_forbidden_default_no_bounds(tested_year: YearNumber) {
        assert!(!DateConstraints::default().is_year_forbidden(tested_year))
    }

    #[test]
    fn picker_config_min_date_greater_than_max_date() {
        let date = from_ymd(2020, 10, 15);
        let config = DateConstraintsBuilder::default()
            .min_date(date.clone())
            .max_date(date.clone() - Duration::days(1))
            .build();
        assert!(config.is_err());
        assert_eq!(
            config.unwrap_err().to_string(),
            "min_date must be earlier or exactly at max_date"
        );
    }

    #[test]
    fn picker_config_min_date_equals_max_date() {
        let date = from_ymd(2020, 10, 15);
        let config = DateConstraintsBuilder::default()
            .min_date(date.clone())
            .max_date(date.clone())
            .build();
        assert!(config.is_ok());
    }

    #[test]
    fn is_day_forbidden_at_min_date_allowed() {
        let date = from_ymd(2020, 10, 15);
        let config = DateConstraintsBuilder::default()
            .min_date(date.clone())
            .build()
            .unwrap();
        assert!(!config.is_day_forbidden(&date))
    }

    #[test]
    fn is_day_forbidden_before_min_date_not_allowed() {
        let date = from_ymd(2020, 10, 15);
        let config = DateConstraintsBuilder::default()
            .min_date(date.clone())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&(date - Duration::days(1))))
    }

    #[test]
    fn is_day_forbidden_at_max_date_allowed() {
        let date = from_ymd(2020, 10, 15);
        let config = DateConstraintsBuilder::default()
            .max_date(date.clone())
            .build()
            .unwrap();
        assert!(!config.is_day_forbidden(&date))
    }

    #[test]
    fn is_day_forbidden_after_max_date_not_allowed() {
        let date = from_ymd(2020, 10, 15);
        let config = DateConstraintsBuilder::default()
            .max_date(date.clone())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&(date + Duration::days(1))))
    }

    #[rstest(
        year => [1, 2000, 3000],
        week => [1, 25, 51],
        disabled_weekday => [Weekday::Mon, Weekday::Tue, Weekday::Sat],
    )]
    fn is_day_forbidden_disabled_weekday_not_allowed(
        year: YearNumber,
        week: u32,
        disabled_weekday: Weekday,
    ) {
        let config = DateConstraintsBuilder::default()
            .disabled_weekdays([disabled_weekday].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(
            &NaiveDate::from_isoywd_opt(year, week, disabled_weekday).expect("invalid date")
        ));
    }

    #[rstest(
        year => [1, 2000, 3000],
        disabled_month => [Month::January, Month::July, Month::December],
        day => [1, 15, 27],
    )]
    fn is_day_forbidden_disabled_month_not_allowed(
        year: YearNumber,
        disabled_month: Month,
        day: DayNumber,
    ) {
        let config = DateConstraintsBuilder::default()
            .disabled_months([disabled_month].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&from_ymd(year, disabled_month.number_from_month(), day)))
    }

    #[rstest(
        disabled_year => [1, 2000, 3000],
        month => [1, 7, 12],
        day => [1, 15, 27],
    )]
    fn is_day_forbidden_disabled_year_not_allowed(
        disabled_year: YearNumber,
        month: MonthNumber,
        day: DayNumber,
    ) {
        let config = DateConstraintsBuilder::default()
            .disabled_years([disabled_year].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&from_ymd(disabled_year, month, day)))
    }

    #[test]
    fn is_day_forbidden_disabled_unique_dates_not_allowed() {
        let date = from_ymd(2020, 1, 16);
        let config = DateConstraintsBuilder::default()
            .disabled_unique_dates([date].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&date))
    }

    #[test]
    fn is_day_forbidden_disabled_unique_dates_after_a_year_allowed() {
        let date = from_ymd(2020, 1, 16);
        let config = DateConstraintsBuilder::default()
            .disabled_unique_dates([date].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(!config.is_day_forbidden(&from_ymd(2021, 1, 16)))
    }

    #[rstest(
        year_in_disabled => [1, 2000, 3000],
        year_in_input => [1, 1500, 2000],
        month => [1, 7, 12],
        day => [1, 15, 27],
    )]
    fn is_day_forbidden_disabled_yearly_dates_not_allowed(
        year_in_disabled: YearNumber,
        year_in_input: YearNumber,
        month: MonthNumber,
        day: DayNumber,
    ) {
        let disabled_yearly_date = from_ymd(year_in_disabled, month, day);
        let config = DateConstraintsBuilder::default()
            .disabled_yearly_dates(vec![disabled_yearly_date])
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&from_ymd(year_in_input, month, day)))
    }

    #[rstest(
        year => [1, 2000, 3000],
        month => [1, 7, 12],
        day => [1, 15, 27],
    )]
    fn is_day_forbidden_disabled_monthly_dates_not_allowed(
        year: YearNumber,
        month: MonthNumber,
        day: DayNumber,
    ) {
        let config = DateConstraintsBuilder::default()
            .disabled_monthly_dates([day].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_day_forbidden(&from_ymd(year, month, day)))
    }

    #[rstest(
        year => [1, 2000, 3000],
        disabled_month => [Month::January, Month::July, Month::December],
        day => [1, 15, 27],
    )]
    fn is_month_forbidden_disabled_months_not_allowed(
        year: YearNumber,
        disabled_month: Month,
        day: DayNumber,
    ) {
        let config = DateConstraintsBuilder::default()
            .disabled_months([disabled_month].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_month_forbidden(&from_ymd(year, disabled_month.number_from_month(), day)))
    }

    #[rstest(
        disabled_year => [1, 2000, 3000],
        month => [1, 7, 12],
        day => [1, 15, 27],
    )]
    fn is_month_forbidden_disabled_years_not_allowed(
        disabled_year: YearNumber,
        month: MonthNumber,
        day: DayNumber,
    ) {
        let config = DateConstraintsBuilder::default()
            .disabled_years([disabled_year].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_month_forbidden(&from_ymd(disabled_year, month, day)))
    }

    #[rstest(
        disabled_year => [1, 2000, 3000],
    )]
    fn is_year_forbidden_disabled_years_not_allowed(disabled_year: YearNumber) {
        let config = DateConstraintsBuilder::default()
            .disabled_years([disabled_year].iter().cloned().collect())
            .build()
            .unwrap();
        assert!(config.is_year_forbidden(disabled_year))
    }

    #[rstest(
        disabled_year_group => [1, 2000, 3000],
    )]
    fn is_year_group_forbidden_disabled_years_not_allowed(disabled_year_group: YearNumber) {
        let config = DateConstraintsBuilder::default()
            .disabled_years(year_group_range(disabled_year_group).collect())
            .build()
            .unwrap();
        assert!(config.is_year_group_forbidden(disabled_year_group))
    }
}
