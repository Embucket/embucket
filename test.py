import sys
import requests
import json
from typing import Any, Generator, Optional, Dict, Tuple

# url = "https://api.embucket.com"
url = "http://localhost:3000"
# filename = "Web_Analytics_sample_events_prepared.csv"
filename = "Web_Analytics_sample_events_140.csv"
volume = sys.argv[1] if len(sys.argv) > 1 else "fs-volume2"
database = sys.argv[2] if len(sys.argv) > 2 else "snowplow2"
schema = sys.argv[3] if len(sys.argv) > 3 else "public"
table = "events_iceberg"

print(sys.argv)

def req(method, handler, payload=None) -> Tuple[Dict, bool, int]:
    print("request:", method, handler, payload if payload is not None else '', end='');
    resp = requests.request(
        method.upper(),
        f'{url}{handler}',
        headers={'Content-Type': 'application/json'},
        data=None if payload is None else json.dumps(payload),
    )
    resp_json = None
    try:
        resp_json = resp.json()
    except:
        pass
    print(f"-> {resp.status_code} {resp_json}")
    return resp_json if resp.ok else None, resp.ok, resp.status_code

def create_volume(payload) -> str:
    name = payload['name']
    volumes, ok, _ = req('get', '/ui/volumes')
    matched_volumes = []
    if ok:
        print(f"volumes {volumes}")
        matched_volumes = list(filter( lambda x: x["name"] == name, volumes["items"]))

    if len(matched_volumes) > 0:
        volume = matched_volumes[0]
        print(f'Volume exists: {volume}')
    else:
        volume, create_ok, _ = req('post', '/ui/volumes', payload)
        if create_ok:
            print(f"Volume created: {volume}")
        else:
            raise(Exception(f"Can't create volume: {name}"))
    return payload["name"]

def recreate_database(volume, database) -> str:
    # get existing databases
    databases, ok, code = req('get', "/ui/databases")
    if ok:
        print(f'Existing databases: {databases}')
        wh_exists = list(filter(lambda x: x["name"] == database and x["volume"] == volume, databases["items"]))
        if len(wh_exists) > 0:
            # delete existing
            wh_deleted, delete_ok, delete_code = req('delete', f"/ui/databases/{database}")
            if delete_ok:
                print(f"Database '{database}' deleted {wh_deleted}, code={delete_code}")
    else:
        print(f'Error geting databases, err code:{code}, {databases}')

    # create database
    wh_created, ok, resp = req('post', f"/ui/databases", {
        "volume": volume,
        "name": database,
    })
    if ok:
        print(f"Created database: {wh_created["name"]}")
    else:
        raise(Exception(f"Can't create database {database}"))
    return f"{volume}/{database}"

def recreate_schema(database, schema):
    print("recreate_schema", schema)
    _, ok, code = req('get', f'/ui/databases/{database}/schemas/{schema}')
    if ok:
        del_schema, ok, _ = req('delete', f'/ui/databases/{database}/schemas/{schema}')
        print(f'delete "{schema}" schema {ok} {del_schema}')
    else:
        print(f'Error geting schema "{schema}", err code:{code}')

    _, ok, _ = req('post', f'/ui/databases/{database}/schemas', { "name": schema }
                   )
    print(f'Created "{schema}" in database {database}')


def create_worksheet(worksheet_payload):
    print("create_worksheet if not exists")

    # get existing worksheets
    worksheets, ok, code = req('get', "/ui/worksheets")
    if ok:
        print(f'worksheets: {worksheets}')
        worksheets_exists = list(filter( lambda x: x, worksheets['items']))
        print(f'Existing worksheets: {worksheets_exists}')
        if len(worksheets_exists):
            return worksheets_exists[0]
    else:
        print(f'Error geting worksheets, err code:{code}')

    created_worksheet, ok, code = req('post', '/ui/worksheets', worksheet_payload)
    if ok:
        print('Created worksheet: {created_worksheet}')
    else:
        raise(Exception('Can\'t create worksheet'))
    return created_worksheet


# fs_volume = create_volume({
#     "name": volume,
#     "type": "file",
#     "path": "data4",
# })

# fs_volume = create_volume({
#     "name": volume,
#     "type": "memory",
# })


# print(recreate_database(fs_volume, database)) # for upload
# print(recreate_schema(database, schema)) # for upload
#
# print(recreate_database(fs_volume, 'db1'))
# print(recreate_database(fs_volume, 'db2'))


# print(recreate_schema('db1', "schema2"))
#
# s3_volume = create_volume({
#     "name": "ice-bucket-volume",
#     "type": "s3",
#     "bucket": "icebucket-for-ui",
#     "endpoint": "http://localhost:9000",
#     "skip-signature": None,
#     "metadata-endpoint": None,
#     "credentials": {
#         "credential_type": "access_key",
#         "aws-access-key-id": "kPYGGu34jF685erC7gst",
#         "aws-secret-access-key": "Q2ClWJgwIZLcX4IE2zO2GBl8qXz7g4knqwLwUpWL"
#     }
# })
#
# try:
#     print(recreate_database('bad volume', 'db2'))
# except Exception as e:
#     print(e)
#
# try:
#     # expect error: Already exists
#     print(recreate_database(s3_volume, 'db2'))
# except Exception as e:
#     print(e)
#
# print(recreate_database(s3_volume, 'db3'))
# print(recreate_database(s3_volume, 'db4'))
#
# print(recreate_schema('db3', schema))
# print(recreate_schema('db4', "schema2"))
#
worksheet = create_worksheet({
    "name": "",
    "content": "",
})

_, query_ok, query_resp = req("post", f"/ui/queries?worksheet_id={worksheet["id"]}", {
    "query": f"""
    SELECT 1, 1.2, 'text text text text text text text', [1,2,3,4,5];
"""
})
print(f"{query_ok}, {query_resp}")
#
# _, query_ok, query_resp = req("post", f"/ui/queries?worksheet_id={worksheet["id"]}", {
#     "query": f"""SELECT
#         id,
#         name,
#         RANDOM() AS random_value,
#         CURRENT_TIMESTAMP AS current_time
#     FROM (VALUES
#         (1, 'Alice'),
#         (2, 'Bob'),
#         (3, 'Charlie'),
#         (4, 'David')
#     ) AS t(id, name);"""
# })
# print(f"{query_ok}, {query_resp}")
#
# _, query_ok, query_resp = req("post", f"/ui/queries", {
#     "query": f"""SELECT
#         id,
#         name,
#         RANDOM() AS random_value,
#         CURRENT_TIMESTAMP AS current_time
#     FROM (VALUES
#         (1, 'Alice'),
#         (2, 'Bob'),
#         (3, 'Charlie'),
#         (4, 'David')
#     ) AS t(id, name);"""
# })
# print(f"{query_ok}, {query_resp}")
#
# ### Query csv table (non existing yet)
# _, query_ok, query_resp = req("post", f"/ui/queries", {
#     "query": f"""SELECT count(*) from {database}.{schema}.{table};"""
# })
# print(f"{query_ok}, {query_resp}")
#
# _, query_ok, query_resp = req("post", f"/ui/queries", {
#     "query": f"""create or replace TABLE {database}.{schema}.{table}3  (
# 	    APP_ID TEXT,
# 	    PLATFORM TEXT,
# 	    EVENT TEXT,
# 	    EVENT_ID TEXT);"""
# })
# print(f"{query_ok}, {query_resp}")
#
# _, query_ok, query_resp = req("post", f"/ui/queries", {
#     "query": f"""COMMENT ON TABLE {database}.{schema}.{table}3 IS 'table 3';"""
# })
# print(f"{query_ok}, {query_resp}")
#
# _, query_ok, query_resp = req("post", f"/ui/queries", {
#     "query": f"""create or replace Iceberg TABLE {database}.{schema}.{table}4
#         external_volume = ''
# 	    catalog = ''
# 	    base_location = ''
#         (
# 	    APP_ID TEXT,
# 	    PLATFORM TEXT,
# 	    ETL_TSTAMP TIMESTAMP_NTZ(9),
# 	    COLLECTOR_TSTAMP TIMESTAMP_NTZ(9) NOT NULL,
# 	    DVCE_CREATED_TSTAMP TIMESTAMP_NTZ(9),
# 	    EVENT TEXT,
# 	    EVENT_ID TEXT);"""
# })
# print(f"{query_ok}, {query_resp}")
#
# ### Query csv table (non existing yet)
# _, query_ok, query_resp = req("post", f"/ui/queries", {
#     "query": f"""SELECT count(*) from {database}.{schema}.{table};"""
# })
# print(f"{query_ok}, {query_resp}")
#
# ### UPLOAD TABLE
#
# # query = f"""create or replace Iceberg TABLE {database}.{schema}.{table}
# #     external_volume = ''
# # 	catalog = ''
# # 	base_location = ''
# #     (
# # 	APP_ID TEXT,
# # 	PLATFORM TEXT,
# # 	ETL_TSTAMP TIMESTAMP_NTZ(9),
# # 	COLLECTOR_TSTAMP TIMESTAMP_NTZ(9) NOT NULL,
# # 	DVCE_CREATED_TSTAMP TIMESTAMP_NTZ(9),
# # 	EVENT TEXT,
# # 	EVENT_ID TEXT,
# # 	TXN_ID NUMBER(38,0),
# # 	NAME_TRACKER TEXT,
# # 	V_TRACKER TEXT,
# # 	V_COLLECTOR TEXT,
# # 	V_ETL TEXT,
# # 	USER_ID TEXT,
# # 	USER_IPADDRESS TEXT,
# # 	USER_FINGERPRINT TEXT,
# # 	DOMAIN_USERID TEXT,
# # 	DOMAIN_SESSIONIDX NUMBER(38,0),
# # 	NETWORK_USERID TEXT,
# # 	GEO_COUNTRY TEXT,
# # 	GEO_REGION TEXT,
# # 	GEO_CITY TEXT,
# # 	GEO_ZIPCODE TEXT,
# # 	GEO_LATITUDE FLOAT,
# # 	GEO_LONGITUDE FLOAT,
# # 	GEO_REGION_NAME TEXT,
# # 	IP_ISP TEXT,
# # 	IP_ORGANIZATION TEXT,
# # 	IP_DOMAIN TEXT,
# # 	IP_NETSPEED TEXT,
# # 	PAGE_URL TEXT,
# # 	PAGE_TITLE TEXT,
# # 	PAGE_REFERRER TEXT,
# # 	PAGE_URLSCHEME TEXT,
# # 	PAGE_URLHOST TEXT,
# # 	PAGE_URLPORT NUMBER(38,0),
# # 	PAGE_URLPATH TEXT,
# # 	PAGE_URLQUERY TEXT,
# # 	PAGE_URLFRAGMENT TEXT,
# # 	REFR_URLSCHEME TEXT,
# # 	REFR_URLHOST TEXT,
# # 	REFR_URLPORT NUMBER(38,0),
# # 	REFR_URLPATH TEXT,
# # 	REFR_URLQUERY TEXT,
# # 	REFR_URLFRAGMENT TEXT,
# # 	REFR_MEDIUM TEXT,
# # 	REFR_SOURCE TEXT,
# # 	REFR_TERM TEXT,
# # 	MKT_MEDIUM TEXT,
# # 	MKT_SOURCE TEXT,
# # 	MKT_TERM TEXT,
# # 	MKT_CONTENT TEXT,
# # 	MKT_CAMPAIGN TEXT,
# # 	SE_CATEGORY TEXT,
# # 	SE_ACTION TEXT,
# # 	SE_LABEL TEXT,
# # 	SE_PROPERTY TEXT,
# # 	SE_VALUE FLOAT,
# # 	TR_ORDERID TEXT,
# # 	TR_AFFILIATION TEXT,
# # 	TR_TOTAL NUMBER(18,2),
# # 	TR_TAX NUMBER(18,2),
# # 	TR_SHIPPING NUMBER(18,2),
# # 	TR_CITY TEXT,
# # 	TR_STATE TEXT,
# # 	TR_COUNTRY TEXT,
# # 	TI_ORDERID TEXT,
# # 	TI_SKU TEXT,
# # 	TI_NAME TEXT,
# # 	TI_CATEGORY TEXT,
# # 	TI_PRICE NUMBER(18,2),
# # 	TI_QUANTITY NUMBER(38,0),
# # 	PP_XOFFSET_MIN NUMBER(38,0),
# # 	PP_XOFFSET_MAX NUMBER(38,0),
# # 	PP_YOFFSET_MIN NUMBER(38,0),
# # 	PP_YOFFSET_MAX NUMBER(38,0),
# # 	USERAGENT TEXT,
# # 	BR_NAME TEXT,
# # 	BR_FAMILY TEXT,
# # 	BR_VERSION TEXT,
# # 	BR_TYPE TEXT,
# # 	BR_RENDERENGINE TEXT,
# # 	BR_LANG TEXT,
# # 	BR_FEATURES_PDF BOOLEAN,
# # 	BR_FEATURES_FLASH BOOLEAN,
# # 	BR_FEATURES_JAVA BOOLEAN,
# # 	BR_FEATURES_DIRECTOR BOOLEAN,
# # 	BR_FEATURES_QUICKTIME BOOLEAN,
# # 	BR_FEATURES_REALPLAYER BOOLEAN,
# # 	BR_FEATURES_WINDOWSMEDIA BOOLEAN,
# # 	BR_FEATURES_GEARS BOOLEAN,
# # 	BR_FEATURES_SILVERLIGHT BOOLEAN,
# # 	BR_COOKIES BOOLEAN,
# # 	BR_COLORDEPTH TEXT,
# # 	BR_VIEWWIDTH NUMBER(38,0),
# # 	BR_VIEWHEIGHT NUMBER(38,0),
# # 	OS_NAME TEXT,
# # 	OS_FAMILY TEXT,
# # 	OS_MANUFACTURER TEXT,
# # 	OS_TIMEZONE TEXT,
# # 	DVCE_TYPE TEXT,
# # 	DVCE_ISMOBILE BOOLEAN,
# # 	DVCE_SCREENWIDTH NUMBER(38,0),
# # 	DVCE_SCREENHEIGHT NUMBER(38,0),
# # 	DOC_CHARSET TEXT,
# # 	DOC_WIDTH NUMBER(38,0),
# # 	DOC_HEIGHT NUMBER(38,0),
# # 	TR_CURRENCY TEXT,
# # 	TR_TOTAL_BASE NUMBER(18,2),
# # 	TR_TAX_BASE NUMBER(18,2),
# # 	TR_SHIPPING_BASE NUMBER(18,2),
# # 	TI_CURRENCY TEXT,
# # 	TI_PRICE_BASE NUMBER(18,2),
# # 	BASE_CURRENCY TEXT,
# # 	GEO_TIMEZONE TEXT,
# # 	MKT_CLICKID TEXT,
# # 	MKT_NETWORK TEXT,
# # 	ETL_TAGS TEXT,
# # 	DVCE_SENT_TSTAMP TIMESTAMP_NTZ(9),
# # 	REFR_DOMAIN_USERID TEXT,
# # 	REFR_DVCE_TSTAMP TIMESTAMP_NTZ(9),
# # 	DOMAIN_SESSIONID TEXT,
# # 	DERIVED_TSTAMP TIMESTAMP_NTZ(9),
# # 	EVENT_VENDOR TEXT,
# # 	EVENT_NAME TEXT,
# # 	EVENT_FORMAT TEXT,
# # 	EVENT_VERSION TEXT,
# # 	EVENT_FINGERPRINT TEXT,
# # 	TRUE_TSTAMP TIMESTAMP_NTZ(9),
# # 	LOAD_TSTAMP TIMESTAMP_NTZ(9),
# # 	CONTEXTS_COM_SNOWPLOWANALYTICS_SNOWPLOW_UA_PARSER_CONTEXT_1 TEXT,
# # 	CONTEXTS_COM_SNOWPLOWANALYTICS_SNOWPLOW_WEB_PAGE_1 TEXT,
# # 	CONTEXTS_COM_IAB_SNOWPLOW_SPIDERS_AND_ROBOTS_1 TEXT,
# # 	CONTEXTS_NL_BASJES_YAUAA_CONTEXT_1 TEXT,
# # 	constraint EVENT_ID_PK primary key (EVENT_ID)
# # );""".replace(
# #     "\n", ""
# # ).replace(
# #     "	", " "
# # )
#
# # ## QUERY CREATE TABLE EVENTS
# # _, table_ok, table_code = req("post", f"/ui/queries?worksheet_id={worksheet["id"]}", {
# #     "query": query,
# # })
# # if table_ok:
# #     print("table  created")
# # else:
# #     print(f"table creation error code={table_code}")
# #     exit(1)
#
#
# ### UPLOAD TINY DATA
#
# tiny_csv = "tiny.csv"
# response = requests.post(
#     f"{url}/ui/databases/{database}/schemas/{schema}/tables/tiny2/rows?header=true",
#     files=[
#         ("uploadFile", (tiny_csv, open(tiny_csv, "rb"), "text/csv")),
#     ],
# )
#
# print(f"upload resp: {response.content}");
# response.raise_for_status()
#
#
# ### UPLOAD DATA
#
# limit_no_timestamps = "Web_Analytics_sample_events_limit_140_no_timestamps.csv"
# response = requests.post(
#     f"{url}/ui/databases/{database}/schemas/{schema}/tables/{table}_140_no_ts/rows?header=true",
#     files=[
#         ("uploadFile", (limit_no_timestamps, open(limit_no_timestamps, "rb"), "text/csv")),
#         #("uploadFile", (filename, open(filename, "rb"), "text/csv")),
#         # ("uploadFile2", (filename, open(filename, "rb"), "text/csv")),
#         # ("uploadFile3", (filename, open(filename, "rb"), "text/csv")),
#     ],
# )
#
# print(f"upload resp: {response.content}");
# response.raise_for_status()
#
# limit_no_timestamps = "Web_Analytics_sample_events_limit_140.csv"
# response = requests.post(
#     f"{url}/ui/databases/{database}/schemas/{schema}/tables/{table}_140/rows?header=true",
#     files=[
#         ("uploadFile", (limit_no_timestamps, open(limit_no_timestamps, "rb"), "text/csv")),
#         #("uploadFile", (filename, open(filename, "rb"), "text/csv")),
#         # ("uploadFile2", (filename, open(filename, "rb"), "text/csv")),
#         # ("uploadFile3", (filename, open(filename, "rb"), "text/csv")),
#     ],
# )
#
# print(f"upload resp: {response.content}");
# response.raise_for_status()
#
#
# # _, query_ok, query_resp = req("post", f"/ui/queries", {
# #     "query": f"""SELECT column_name, data_type FROM datafusion.information_schema.columns WHERE table_name = '{table}' AND table_catalog = '{database}' AND table_schema = '{schema}';"""
# #     # f"""SELECT * FROM datafusion.information_schema.columns limit 1;"""
# # })
# # print(f"{query_ok}, {query_resp}")
#
# # ### Query csv table
# # _, query_ok, query_resp = req("post", f"/ui/queries", {
# #     "query": f"""SELECT * from {database}.{schema}.{table} LIMIT 10;"""
# # })
# # print(f"{query_ok}, {query_resp}")