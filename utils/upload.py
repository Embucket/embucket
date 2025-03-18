import sys
import requests
import json
from typing import Any, Generator, Optional, Dict, Tuple

url = "http://0.0.0.0:3000"
filename = "Web_Analytics_sample_events_prepared.csv"
bucket = sys.argv[1] if len(sys.argv) > 1 else "fsbucket"
catalog = sys.argv[2] if len(sys.argv) > 2 else "snowplow"
schema = sys.argv[3] if len(sys.argv) > 3 else "public"
# table = "sample_events_web_base"
table = "events_iceberg"

volume_name = None
database_name = None

print(sys.argv)

def req(method, handler, payload=None) -> Tuple[Dict, bool, int]:
    print("request:", method, handler, payload if payload is not None else '', end='');
    resp = requests.request(
        method.upper(),
        f'{url}{handler}',
        headers={'Content-Type': 'application/json'},
        data=None if payload is None else json.dumps(payload)
    )
    print(f"-> {resp.status_code}")
    # print("resp", resp.ok, resp.status_code);
    resp_json = None
    try:
        resp_json = resp.json()
    except:
        pass
    return resp_json if resp.ok else None, resp.ok, resp.status_code

def create_storage_volume() -> str:
    volume = None
    volumes, ok, _ = req('get', '/ui/volumes')
    matched_volumes = []
    if ok:
        matched_volumes = list(filter( lambda x: x["ident"] == bucket, volumes))

    if len(matched_volumes) > 0:
        volume = matched_volumes[0]
        print(f'Get exisiting volume {volume["ident"]}')
    else:
        volume, create_ok, _ = req('post', '/ui/volumes', {
            "ident": "fsbucket",
            "type": "file",
            "path": "/tmp",
            # "type": "aws",
            # "region": "us-west-1",
            # "bucket": bucket,
            # "credentials": {
            #     "credential_type": "access_key",
            #     "aws_access_key_id": "TIF4SXwsJzmejxsL9mer",
            #     "aws_secret_access_key": "fZLdl7reDuHKCQkd4KU39fAYVogHVjvJHbmjaw5N"
            # },
            # "endpoint": f"http://localhost:9000",
        })
        if create_ok:
            print(f"Created Icehut volume_name: {volume}")
        else:
            raise(Exception("Can't create Icehut volume_name"))
    print(f"Use volume: {volume}")
    return volume["ident"]

def recreate_database(catalog) -> str:
    database_name = None
    name = catalog
    if database_name is None:
        # get existing database_id
        databases, ok, code = req('get', "/ui/databases")
        if ok:
            print(f'Get existing databases {databases}')
            wh_exists = list(filter(lambda x: x["ident"] == catalog, databases))
            if len(wh_exists) > 0:
                database_name = wh_exists[0]["ident"]
        else:
            print(f'Error geting databases, err code:{code}')
    if database_name is not None:
        wh_deleted, delete_ok, delete_code = req('delete', f"/ui/databases/{database_name}")
        if delete_ok:
            print(f"Icehut database '{catalog}' deleted {wh_deleted}, code={delete_code}")

    # create database
    wh_created, ok, resp = req('post', f"/ui/databases", {
        "volume": volume_name,
        "ident": catalog,
    })
    if ok:
        database_name = wh_created["ident"]
        print(f"Created database: {wh_created}")
    else:
        raise(Exception(f"Can't create database"))
    return database_name

def recreate_schema(schema):
    print("recreate_schema", schema)
    _, ok, code = req('get', f'/ui/databases/{database_name}/schemas/{schema}')
    if ok:
        del_schema, ok, _ = req('delete', f'/ui/databases/{database_name}/schemas/{schema}')
        print(f'delete "{schema}" schema {ok} {del_schema}')
    else:
        print(f'Error geting schema "{schema}", err code:{code}')

    _, ok, _ = req('post', f'/ui/databases/{database_name}/schemas', {
        "ident": {
            "schema": schema,
            "database": database_name,
        }
    })
    print(f'create "{schema}" database {ok}')


### VOLUME_NAME
volume_name = create_storage_volume()

## DATABASE
database_name = recreate_database(catalog)
recreate_schema(schema)

query = f"""create or replace Iceberg TABLE {catalog}.{schema}.{table} 
    external_volume = ''
	catalog = ''
	base_location = '' 
    (
	APP_ID TEXT,
	PLATFORM TEXT,
	ETL_TSTAMP TIMESTAMP_NTZ(9),
	COLLECTOR_TSTAMP TIMESTAMP_NTZ(9) NOT NULL,
	DVCE_CREATED_TSTAMP TIMESTAMP_NTZ(9),
	EVENT TEXT,
	EVENT_ID TEXT,
	TXN_ID NUMBER(38,0),
	NAME_TRACKER TEXT,
	V_TRACKER TEXT,
	V_COLLECTOR TEXT,
	V_ETL TEXT,
	USER_ID TEXT,
	USER_IPADDRESS TEXT,
	USER_FINGERPRINT TEXT,
	DOMAIN_USERID TEXT,
	DOMAIN_SESSIONIDX NUMBER(38,0),
	NETWORK_USERID TEXT,
	GEO_COUNTRY TEXT,
	GEO_REGION TEXT,
	GEO_CITY TEXT,
	GEO_ZIPCODE TEXT,
	GEO_LATITUDE FLOAT,
	GEO_LONGITUDE FLOAT,
	GEO_REGION_NAME TEXT,
	IP_ISP TEXT,
	IP_ORGANIZATION TEXT,
	IP_DOMAIN TEXT,
	IP_NETSPEED TEXT,
	PAGE_URL TEXT,
	PAGE_TITLE TEXT,
	PAGE_REFERRER TEXT,
	PAGE_URLSCHEME TEXT,
	PAGE_URLHOST TEXT,
	PAGE_URLPORT NUMBER(38,0),
	PAGE_URLPATH TEXT,
	PAGE_URLQUERY TEXT,
	PAGE_URLFRAGMENT TEXT,
	REFR_URLSCHEME TEXT,
	REFR_URLHOST TEXT,
	REFR_URLPORT NUMBER(38,0),
	REFR_URLPATH TEXT,
	REFR_URLQUERY TEXT,
	REFR_URLFRAGMENT TEXT,
	REFR_MEDIUM TEXT,
	REFR_SOURCE TEXT,
	REFR_TERM TEXT,
	MKT_MEDIUM TEXT,
	MKT_SOURCE TEXT,
	MKT_TERM TEXT,
	MKT_CONTENT TEXT,
	MKT_CAMPAIGN TEXT,
	SE_CATEGORY TEXT,
	SE_ACTION TEXT,
	SE_LABEL TEXT,
	SE_PROPERTY TEXT,
	SE_VALUE FLOAT,
	TR_ORDERID TEXT,
	TR_AFFILIATION TEXT,
	TR_TOTAL NUMBER(18,2),
	TR_TAX NUMBER(18,2),
	TR_SHIPPING NUMBER(18,2),
	TR_CITY TEXT,
	TR_STATE TEXT,
	TR_COUNTRY TEXT,
	TI_ORDERID TEXT,
	TI_SKU TEXT,
	TI_NAME TEXT,
	TI_CATEGORY TEXT,
	TI_PRICE NUMBER(18,2),
	TI_QUANTITY NUMBER(38,0),
	PP_XOFFSET_MIN NUMBER(38,0),
	PP_XOFFSET_MAX NUMBER(38,0),
	PP_YOFFSET_MIN NUMBER(38,0),
	PP_YOFFSET_MAX NUMBER(38,0),
	USERAGENT TEXT,
	BR_NAME TEXT,
	BR_FAMILY TEXT,
	BR_VERSION TEXT,
	BR_TYPE TEXT,
	BR_RENDERENGINE TEXT,
	BR_LANG TEXT,
	BR_FEATURES_PDF BOOLEAN,
	BR_FEATURES_FLASH BOOLEAN,
	BR_FEATURES_JAVA BOOLEAN,
	BR_FEATURES_DIRECTOR BOOLEAN,
	BR_FEATURES_QUICKTIME BOOLEAN,
	BR_FEATURES_REALPLAYER BOOLEAN,
	BR_FEATURES_WINDOWSMEDIA BOOLEAN,
	BR_FEATURES_GEARS BOOLEAN,
	BR_FEATURES_SILVERLIGHT BOOLEAN,
	BR_COOKIES BOOLEAN,
	BR_COLORDEPTH TEXT,
	BR_VIEWWIDTH NUMBER(38,0),
	BR_VIEWHEIGHT NUMBER(38,0),
	OS_NAME TEXT,
	OS_FAMILY TEXT,
	OS_MANUFACTURER TEXT,
	OS_TIMEZONE TEXT,
	DVCE_TYPE TEXT,
	DVCE_ISMOBILE BOOLEAN,
	DVCE_SCREENWIDTH NUMBER(38,0),
	DVCE_SCREENHEIGHT NUMBER(38,0),
	DOC_CHARSET TEXT,
	DOC_WIDTH NUMBER(38,0),
	DOC_HEIGHT NUMBER(38,0),
	TR_CURRENCY TEXT,
	TR_TOTAL_BASE NUMBER(18,2),
	TR_TAX_BASE NUMBER(18,2),
	TR_SHIPPING_BASE NUMBER(18,2),
	TI_CURRENCY TEXT,
	TI_PRICE_BASE NUMBER(18,2),
	BASE_CURRENCY TEXT,
	GEO_TIMEZONE TEXT,
	MKT_CLICKID TEXT,
	MKT_NETWORK TEXT,
	ETL_TAGS TEXT,
	DVCE_SENT_TSTAMP TIMESTAMP_NTZ(9),
	REFR_DOMAIN_USERID TEXT,
	REFR_DVCE_TSTAMP TIMESTAMP_NTZ(9),
	DOMAIN_SESSIONID TEXT,
	DERIVED_TSTAMP TIMESTAMP_NTZ(9),
	EVENT_VENDOR TEXT,
	EVENT_NAME TEXT,
	EVENT_FORMAT TEXT,
	EVENT_VERSION TEXT,
	EVENT_FINGERPRINT TEXT,
	TRUE_TSTAMP TIMESTAMP_NTZ(9),
	LOAD_TSTAMP TIMESTAMP_NTZ(9),
	CONTEXTS_COM_SNOWPLOWANALYTICS_SNOWPLOW_UA_PARSER_CONTEXT_1 TEXT,
	CONTEXTS_COM_SNOWPLOWANALYTICS_SNOWPLOW_WEB_PAGE_1 TEXT,
	CONTEXTS_COM_IAB_SNOWPLOW_SPIDERS_AND_ROBOTS_1 TEXT,
	CONTEXTS_NL_BASJES_YAUAA_CONTEXT_1 TEXT,
	constraint EVENT_ID_PK primary key (EVENT_ID)
);""".replace(
    "\n", ""
).replace(
    "	", " "
)

## QUERY CREATE TABLE EVENTS
_, table_ok, table_code = req("post", f"/ui/query", {
    "query": query,
})
if table_ok:
    print("table  created")
else:
    print(f"table creation error code={table_code}")
    exit(1)

# response = requests.post(
#     f"{url}/ui/query",
#     json={
#         "query": query,
#     },
# )


# response.raise_for_status()


# UPLOAD DATA
files = [("uploadFile", (filename, open(filename, "rb"), "text/csv"))]

response = requests.post(
    f"{url}/ui/databases/{database_name}/schemas/{schema}/tables/{table}/upload",
    files=files,
)

response.raise_for_status()