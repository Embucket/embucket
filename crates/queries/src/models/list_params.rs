use super::query_source::QuerySource;
use super::query_status::QueryStatus;
use super::result_format::ResultFormat;

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub enum OrderBy {
    Status(OrderDirection),
    Source(OrderDirection),
    Format(OrderDirection),
    // we reuse QueryStatus just to point timestamp we want to sort by
    // as every status is directly linked to corresponding timestamp
    Timestamp(OrderDirection, QueryStatus),
    Duration(OrderDirection),
    RowsCount(OrderDirection),
    Error(OrderDirection),
}

#[derive(Debug, Clone)]
pub enum FilterBy {
    Status(QueryStatus),
    Source(QuerySource),
    Format(ResultFormat),
    Sql(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ListParams {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub filter_by: Vec<FilterBy>,
    pub order_by: Vec<OrderBy>,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            offset: None,
            limit: None,
            filter_by: vec![],
            order_by: vec![OrderBy::Timestamp(
                OrderDirection::Desc,
                QueryStatus::Created,
            )],
        }
    }
}

impl ListParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_offset(self, offset: i64) -> Self {
        Self {
            offset: Some(offset),
            ..self
        }
    }
    #[must_use]
    pub fn with_limit(self, limit: i64) -> Self {
        Self {
            limit: Some(limit),
            ..self
        }
    }
    #[must_use]
    pub fn with_filter_by(self, filter_by: Vec<FilterBy>) -> Self {
        Self { filter_by, ..self }
    }
    #[must_use]
    pub fn with_order_by(self, order_by: Vec<OrderBy>) -> Self {
        Self { order_by, ..self }
    }
}
