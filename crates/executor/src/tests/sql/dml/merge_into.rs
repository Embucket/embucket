use crate::test_query;


test_query!(
    merge_into_only_update,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row')",
        "INSERT INTO embucket.public.merge_source VALUES (1, 'updated row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET merge_target.description = merge_source.description",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_insert_and_update,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "INSERT INTO embucket.public.merge_source VALUES (2, 'updated row'), (3, 'new row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_empty_source,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_ctas_source_multi_insert_target,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row'), (3, 'existing row'), (4, 'existing row'), (5, 'existing row')",
        "INSERT INTO embucket.public.merge_target VALUES (6, 'existing row'), (7, 'existing row'), (8, 'existing row'), (9, 'existing row')",
        "INSERT INTO embucket.public.merge_target VALUES (11, 'existing row'), (12, 'existing row'), (13, 'existing row'), (14, 'existing row'), (15, 'existing row')",
        "INSERT INTO embucket.public.merge_target VALUES (16, 'existing row'), (17, 'existing row'), (18, 'existing row'), (19, 'existing row')",
        "CREATE OR REPLACE TABLE embucket.public.merge_source AS SELECT column1 as id, column2 as description FROM VALUES (1, 'updated row'), (10, 'new row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
        "CREATE OR REPLACE TABLE embucket.public.merge_source AS SELECT column1 as id, column2 as description FROM VALUES (11, 'updated row'), (20, 'new row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_ambigious_insert,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "INSERT INTO embucket.public.merge_source VALUES (2, 'updated row'), (3, 'new row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_insert_and_update_alias,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "INSERT INTO embucket.public.merge_source VALUES (2, 'updated row'), (3, 'new row')",
        "MERGE INTO merge_target t USING merge_source s ON t.id = s.id WHEN MATCHED THEN UPDATE SET description = s.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (s.id, s.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_with_predicate,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "INSERT INTO embucket.public.merge_source VALUES (2, 'updated row'), (3, 'new row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED AND merge_target.id = 1 THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_with_partition_filter_preserves_unmatched_rows,
    "SELECT
        COUNT(*) as total_rows,
        COUNT(CASE WHEN event_time >= '2024-06-01' AND event_time < '2024-07-01' THEN 1 END) as june_rows,
        COUNT(CASE WHEN event_time < '2024-06-01' THEN 1 END) as before_june_rows,
        COUNT(CASE WHEN description = 'january data' THEN 1 END) as january_preserved,
        COUNT(CASE WHEN description = 'updated june data' THEN 1 END) as june_updated,
        COUNT(CASE WHEN description = 'new june data' THEN 1 END) as june_inserted
    FROM embucket.public.events_target",
    setup_queries = [
        "CREATE TABLE embucket.public.events_target (ID INTEGER, event_time TIMESTAMP, description VARCHAR)",
        "CREATE TABLE embucket.public.events_source (ID INTEGER, event_time TIMESTAMP, description VARCHAR)",
        "INSERT INTO embucket.public.events_target VALUES (1, '2024-01-15 10:00:00', 'january data'), (2, '2024-01-20 14:30:00', 'january data'), (3, '2024-06-15 09:00:00', 'original june data'), (4, '2024-06-20 16:45:00', 'original june data')",
        "INSERT INTO embucket.public.events_source VALUES (3, '2024-06-15 09:00:00', 'updated june data'), (5, '2024-06-25 11:30:00', 'new june data')",
        "MERGE INTO events_target t USING events_source s ON t.id = s.id AND t.event_time >= CAST('2024-06-01' AS TIMESTAMP) AND t.event_time < CAST('2024-07-01' AS TIMESTAMP) WHEN MATCHED THEN UPDATE SET t.description = s.description WHEN NOT MATCHED THEN INSERT (id, event_time, description) VALUES (s.id, s.event_time, s.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_between_timestamp,
    "SELECT
        COUNT(*) as total_rows,
        MIN(start_tstamp) as min_start,
        MAX(start_tstamp) as max_start,
        COUNT(CASE WHEN start_tstamp < '2024-06-01' THEN 1 END) as sessions_before_june,
        COUNT(CASE WHEN session_id = 'jan_session_1' THEN 1 END) as jan_1_preserved,
        COUNT(CASE WHEN session_id = 'may_session_1' THEN 1 END) as may_1_preserved,
        COUNT(CASE WHEN session_id = 'jan_session_1' AND start_tstamp >= '2024-06-01' THEN 1 END) as jan_new
    FROM embucket.public.lifecycle_manifest",
    setup_queries = [
        "CREATE TABLE embucket.public.lifecycle_manifest (session_id VARCHAR, start_tstamp TIMESTAMP, end_tstamp TIMESTAMP)",
        "CREATE TABLE embucket.public.lifecycle_source (session_id VARCHAR, start_tstamp TIMESTAMP, end_tstamp TIMESTAMP)",
        "INSERT INTO embucket.public.lifecycle_manifest VALUES ('jan_session_1', '2024-01-15 10:00:00', '2024-01-15 11:00:00'), ('jan_session_2', '2024-01-20 14:00:00', '2024-01-20 15:00:00'), ('may_session_1', '2024-05-10 09:00:00', '2024-05-10 10:00:00'), ('may_session_2', '2024-05-15 12:00:00', '2024-05-15 13:00:00')",
        "INSERT INTO embucket.public.lifecycle_source VALUES ('jan_session_1', '2024-06-15 09:00:00', '2024-06-15 10:00:00'), ('may_session_1', '2024-06-20 11:00:00', '2024-06-20 12:00:00')",
        "MERGE INTO lifecycle_manifest t USING lifecycle_source s ON (t.start_tstamp BETWEEN CAST('2024-04-01' AS TIMESTAMP) AND CAST('2024-12-31' AS TIMESTAMP)) AND (s.session_id = t.session_id) WHEN MATCHED THEN UPDATE SET t.start_tstamp = s.start_tstamp, t.end_tstamp = s.end_tstamp WHEN NOT MATCHED THEN INSERT (session_id, start_tstamp, end_tstamp) VALUES (s.session_id, s.start_tstamp, s.end_tstamp)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_with_values,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "MERGE INTO merge_target USING (SELECT * FROM (VALUES (2, 'updated row'), (3, 'new row')) AS source(id, description)) AS source ON merge_target.id = source.id WHEN MATCHED THEN UPDATE SET description = source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (source.id, source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_empty_table,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_source VALUES (2, 'updated row'), (3, 'new row')",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_from_view,
    "SELECT count(CASE WHEN description = 'updated row' THEN 1 ELSE NULL END) updated, count(CASE WHEN description = 'existing row' THEN 1 ELSE NULL END) existing FROM embucket.public.merge_target",
    setup_queries = [
        "CREATE TABLE embucket.public.merge_target (ID INTEGER, description VARCHAR)",
        "CREATE TABLE embucket.public.merge_source_table (ID INTEGER, description VARCHAR)",
        "INSERT INTO embucket.public.merge_target VALUES (1, 'existing row'), (2, 'existing row')",
        "INSERT INTO embucket.public.merge_source_table VALUES (2, 'updated row'), (3, 'new row')",
        "CREATE VIEW embucket.public.merge_source AS SELECT * FROM embucket.public.merge_source_table",
        "MERGE INTO merge_target USING merge_source ON merge_target.id = merge_source.id WHEN MATCHED THEN UPDATE SET description = merge_source.description WHEN NOT MATCHED THEN INSERT (id, description) VALUES (merge_source.id, merge_source.description)",
    ],
    snapshot_path = "merge_into"
);



test_query!(
    merge_into_column_only_optimization,
    "SELECT * FROM column_only_optimization_target ORDER BY a,b",
    setup_queries = [
        "CREATE TABLE column_only_optimization_target(a int,b string)",
        "CREATE TABLE column_only_optimization_source(a int,b string)",
        "INSERT INTO column_only_optimization_target VALUES(1,'a1'),(2,'a2')",
        "INSERT INTO column_only_optimization_target VALUES(3,'a3'),(4,'a4')",
        "INSERT INTO column_only_optimization_target VALUES(5,'a5'),(6,'a6')",
        "INSERT INTO column_only_optimization_target VALUES(7,'a7'),(8,'a8')",
        "INSERT INTO column_only_optimization_source VALUES(1,'b1'),(2,'b2')",
        "INSERT INTO column_only_optimization_source VALUES(3,'b3'),(4,'b4')",
        "MERGE INTO column_only_optimization_target AS t1 USING column_only_optimization_source AS t2 ON t1.a = t2.a WHEN MATCHED THEN UPDATE SET t1.b = t2.b WHEN NOT MATCHED THEN INSERT (a,b) VALUES (t2.a, t2.b)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_without_distributed_enable,
    "SELECT * FROM t1 ORDER BY a,b,c",
    setup_queries = [
        "CREATE OR REPLACE TABLE t1(a int,b string, c string)",
        "CREATE OR REPLACE TABLE t2(a int,b string, c string)",
        "INSERT INTO t1 VALUES(1,'b1','c1'),(2,'b2','c2')",
        "INSERT INTO t1 VALUES(2,'b3','c3'),(3,'b4','c4')",
        "INSERT INTO t2 VALUES(1,'b_5','c_5'),(3,'b_6','c_6')",
        "INSERT INTO t2 VALUES(2,'b_7','c_7')",
        "MERGE INTO t1 USING (SELECT * FROM t2) AS t2 ON t1.a = t2.a WHEN MATCHED THEN UPDATE SET t1.c = t2.c",
        "INSERT INTO t2 VALUES(4,'b_8','c_8')",
        "MERGE INTO t1 USING (SELECT * FROM t2) AS t2 ON t1.a = t2.a WHEN MATCHED THEN UPDATE SET t1.c = t2.c WHEN NOT MATCHED THEN INSERT (a,b,c) VALUES(t2.a,t2.b,t2.c)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_with_partial_insert,
    "SELECT * FROM t1 ORDER BY a,b,c",
    setup_queries = [
        "CREATE OR REPLACE TABLE t1(a int,b string, c string)",
        "CREATE OR REPLACE TABLE t2(a int,b string, c string)",
        "INSERT INTO t1 VALUES(1,'b1','c1'),(2,'b2','c2')",
        "INSERT INTO t1 VALUES(2,'b3','c3'),(3,'b4','c4')",
        "INSERT INTO t2 VALUES(1,'b_5','c_5'),(3,'b_6','c_6')",
        "INSERT INTO t2 VALUES(2,'b_7','c_7')",
        "INSERT INTO t2 VALUES(4,'b_8','c_8')",
        "MERGE INTO t1 USING (SELECT * FROM t2) AS t2 ON t1.a = t2.a WHEN MATCHED THEN UPDATE SET t1.c = t2.c WHEN NOT MATCHED THEN INSERT (a,c) VALUES(t2.a,t2.c)",
    ],
    snapshot_path = "merge_into"
);

// Test MERGE INTO with empty source scenarios (dbt manifest pattern)
test_query!(
    merge_into_on_false_with_empty_source,
    "SELECT COUNT(*) as row_count FROM embucket.public.manifest",
    setup_queries = [
        "CREATE TABLE embucket.public.manifest (model VARCHAR, last_success TIMESTAMP)",
        "MERGE INTO manifest m USING (SELECT CAST(NULL AS VARCHAR) as model, CAST('1970-01-01' AS TIMESTAMP) as last_success WHERE FALSE) s ON (FALSE) WHEN NOT MATCHED THEN INSERT (model, last_success) VALUES (s.model, s.last_success)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_with_null_aggregate_subquery,
    "SELECT COUNT(*) as row_count FROM embucket.public.manifest",
    setup_queries = [
        "CREATE TABLE embucket.public.manifest (model VARCHAR, last_success TIMESTAMP)",
        "CREATE TABLE embucket.public.base_events (collector_tstamp TIMESTAMP)",
        "MERGE INTO manifest m USING (SELECT 'model_name' as model, a.last_success FROM (SELECT MAX(collector_tstamp) as last_success FROM base_events) a WHERE a.last_success IS NOT NULL) s ON m.model = s.model WHEN MATCHED THEN UPDATE SET last_success = s.last_success WHEN NOT MATCHED THEN INSERT (model, last_success) VALUES (s.model, s.last_success)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_on_false_with_populated_source,
    "SELECT COUNT(*) as row_count FROM embucket.public.manifest",
    setup_queries = [
        "CREATE TABLE embucket.public.manifest (model VARCHAR, last_success TIMESTAMP)",
        "MERGE INTO manifest m USING (SELECT column1 as model, column2 as last_success FROM (VALUES ('model_a', CAST('2025-01-01' AS TIMESTAMP)), ('model_b', CAST('2025-01-02' AS TIMESTAMP)))) s ON (FALSE) WHEN NOT MATCHED THEN INSERT (model, last_success) VALUES (s.model, s.last_success)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_with_aggregate_null_cross_join,
    "SELECT COUNT(*) as row_count FROM embucket.public.manifest",
    setup_queries = [
        "CREATE TABLE embucket.public.manifest (model VARCHAR, last_success TIMESTAMP)",
        "CREATE TABLE embucket.public.base_events (collector_tstamp TIMESTAMP)",
        "MERGE INTO manifest m USING (SELECT b.model, a.last_success FROM (SELECT MAX(collector_tstamp) as last_success FROM base_events) a, (SELECT 'model_1' as model UNION ALL SELECT 'model_2') b WHERE a.last_success IS NOT NULL) s ON m.model = s.model WHEN MATCHED THEN UPDATE SET last_success = GREATEST(m.last_success, s.last_success) WHEN NOT MATCHED THEN INSERT (model, last_success) VALUES (s.model, s.last_success)",
    ],
    snapshot_path = "merge_into"
);

test_query!(
    merge_into_empty_source_with_existing_target_data,
    "SELECT COUNT(*) as row_count, COUNT(CASE WHEN model = 'existing_1' THEN 1 END) as existing_1_count, COUNT(CASE WHEN model = 'existing_2' THEN 1 END) as existing_2_count FROM embucket.public.manifest",
    setup_queries = [
        "CREATE TABLE embucket.public.manifest (model VARCHAR, last_success TIMESTAMP)",
        "INSERT INTO embucket.public.manifest VALUES ('existing_1', CAST('2025-01-01' AS TIMESTAMP))",
        "INSERT INTO embucket.public.manifest VALUES ('existing_2', CAST('2025-01-02' AS TIMESTAMP))",
        "MERGE INTO manifest m USING (SELECT column1 as model, column2 as last_success FROM (VALUES ('x', CAST('2025-01-01' AS TIMESTAMP))) WHERE FALSE) s ON m.model = s.model WHEN MATCHED THEN UPDATE SET last_success = s.last_success WHEN NOT MATCHED THEN INSERT (model, last_success) VALUES (s.model, s.last_success)",
    ],
    snapshot_path = "merge_into"
);
