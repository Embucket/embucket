use serde::Serialize;

// Additional Storage types:
// For lookup without GSIs

// QueryRecord: Query entity persisted in `DynamoDB`.
// pk: String, // QUERY#yyyy-mm-dd
// sk: String, // <uuid_v7>

#[derive(Debug, Serialize)]
pub struct QueryIdLookupItem {
    pub pk: String, // QUERYID#<uuid_v7>
    pub sk: String, // -

    pub ref_pk: String, // QUERY#yyyy-mm-dd
    pub ref_sk: String, // <uuid_v7>
}

#[derive(Debug, Serialize)]
pub struct RequestIdLookupItem {
    pub pk: String, // REQUEST#<uuid_v4>
    pub sk: String, // -

    pub ref_pk: String, // QUERY#yyyy-mm-dd
    pub ref_sk: String, // <uuid_v7>
}
