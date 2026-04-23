use crate::test_query;

test_query!(
    years,
    "SELECT DATEDIFF('year', '2020-04-09 14:39:20'::TIMESTAMP, '2023-05-08 23:39:20'::TIMESTAMP) AS diff_years;",
    snapshot_path = "datediff"
);

test_query!(
    hours,
    "SELECT DATEDIFF('hour',
               '2023-05-08T23:39:20.123-07:00'::TIMESTAMP,
               DATEADD('year', 2, ('2023-05-08T23:39:20.123-07:00')::TIMESTAMP))
    AS diff_hours;",
    snapshot_path = "datediff"
);

test_query!(
    combined,
    "SELECT d,
       DATEDIFF('year', '2017-01-01'::DATE, d) as result_year,
       DATEDIFF('week', '2017-01-01'::DATE, d) as result_week,
       DATEDIFF('day', '2017-01-01'::DATE, d) as result_day,
       DATEDIFF('hour', '2017-01-01'::DATE, d) as result_hour,
       DATEDIFF('minute', '2017-01-01'::DATE, d) as result_minute,
       DATEDIFF('second', '2017-01-01'::DATE, d) as result_second
  FROM VALUES
       ('2016-12-30'::DATE),
       ('2016-12-31'::DATE),
       ('2017-01-01'::DATE),
       ('2017-01-02'::DATE),
       ('2017-01-03'::DATE),
       ('2017-01-04'::DATE),
       ('2017-01-05'::DATE),
       ('2017-12-30'::DATE),
       ('2017-12-31'::DATE)
  AS t(d);",
    snapshot_path = "datediff"
);

test_query!(
    different_types,
    "SELECT
        DATEDIFF('day',
            CAST('2024-08-14 15:30:00' AS TIMESTAMP),
            CAST('2024-08-20' AS DATE)) AS ts_date,
        DATEDIFF('day',
            CAST('2024-08-14' AS DATE),
            CAST('2024-08-20 15:30:00' AS TIMESTAMP)) AS date_ts,
        DATEDIFF('minute',
            CAST('00:10:00' AS TIME),
            CAST('00:15:00' AS TIME)) AS ts_time,
        DATEDIFF('minute',
            '1970-01-01 00:10:00',
            CAST('1970-01-01 00:15:00' AS TIMESTAMP)) AS time_ts,
        DATEDIFF('minute',
            CAST('1970-01-01' AS DATE),
            CAST('1970-02-01 00:15:00' AS TIMESTAMP)) AS date_time",
    snapshot_path = "datediff"
);

// DATEDIFF uses boundary-count semantics (matches Snowflake), not
// ceiling-of-duration. These cases all produce 0 because the endpoints
// sit in the same `part` bucket even though the true duration is positive.
test_query!(
    boundary_count_same_bucket,
    "SELECT
        DATEDIFF('second',
            TIMESTAMP '2020-01-01 00:00:00.100',
            TIMESTAMP '2020-01-01 00:00:00.900') AS sec_sub,
        DATEDIFF('minute',
            TIMESTAMP '2020-01-01 00:00:05',
            TIMESTAMP '2020-01-01 00:00:55') AS min_sub,
        DATEDIFF('hour',
            TIMESTAMP '2020-01-01 01:30:00',
            TIMESTAMP '2020-01-01 01:50:00') AS hour_sub,
        DATEDIFF('day',
            TIMESTAMP '2020-01-01 08:00:00',
            TIMESTAMP '2020-01-01 20:00:00') AS day_sub;",
    snapshot_path = "datediff"
);

// Endpoints straddle a single boundary: DATEDIFF returns 1 even when the
// true elapsed duration is less than one full unit.
test_query!(
    boundary_count_straddle,
    "SELECT
        DATEDIFF('second',
            TIMESTAMP '2020-01-01 00:00:00.900',
            TIMESTAMP '2020-01-01 00:00:01.100') AS sec_straddle,
        DATEDIFF('minute',
            TIMESTAMP '2020-01-01 01:00:55',
            TIMESTAMP '2020-01-01 01:01:05') AS min_straddle,
        DATEDIFF('hour',
            TIMESTAMP '2020-01-01 01:55:00',
            TIMESTAMP '2020-01-01 02:05:00') AS hour_straddle,
        DATEDIFF('day',
            TIMESTAMP '2020-01-01 23:00:00',
            TIMESTAMP '2020-01-02 01:00:00') AS day_straddle;",
    snapshot_path = "datediff"
);

// Counts boundaries, not rounded duration: a 1.5-second span that crosses
// exactly one second-boundary returns 1, not 2 (CEIL(1.5) = 2 would be wrong).
test_query!(
    boundary_count_not_ceiling,
    "SELECT
        DATEDIFF('second',
            TIMESTAMP '2020-01-01 00:00:00.250',
            TIMESTAMP '2020-01-01 00:00:01.750') AS sec_1_5,
        DATEDIFF('second',
            TIMESTAMP '2020-01-01 00:00:00.500',
            TIMESTAMP '2020-01-01 00:00:02.900') AS sec_2_4,
        DATEDIFF('second',
            TIMESTAMP '2020-01-01 00:00:00.000',
            TIMESTAMP '2020-01-01 00:00:02.500') AS sec_2_5,
        DATEDIFF('hour',
            TIMESTAMP '2020-01-01 01:30:00',
            TIMESTAMP '2020-01-01 02:30:00') AS hour_1h_two_buckets;",
    snapshot_path = "datediff"
);
