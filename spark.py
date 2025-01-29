import os
os.environ['SPARK_HOME'] = '/opt/anaconda3/envs/iceberg-lab/lib/python3.12/site-packages/pyspark'

import pyspark
from pyspark.sql import SparkSession

# spark = SparkSession.builder.appName('iceberg_lab') \
# .config('spark.jars.packages', 'org.apache.iceberg:iceberg-spark-runtime-3.5_2.12:1.4.1,software.amazon.awssdk:bundle:2.20.160,software.amazon.awssdk:url-connection-client:2.20.160') \
# .config('spark.sql.extensions', 'org.apache.iceberg.spark.extensions.IcebergSparkSessionExtensions') \
# .config('spark.sql.defaultCatalog', 'opencatalog') \
# .config('spark.sql.catalog.opencatalog', 'org.apache.iceberg.spark.SparkCatalog') \
# .config('spark.sql.catalog.opencatalog.type', 'rest') \
# .config('spark.sql.catalog.opencatalog.header.X-Iceberg-Access-Delegation','vended-credentials') \
# .config('spark.sql.catalog.opencatalog.uri','https://wv37264.us-east-2.aws.snowflakecomputing.com/polaris/api/catalog') \
# .config('spark.sql.catalog.opencatalog.credential','klc4/u9+TVOqsZKCHcoin0HgSLw=:CI4ktHhqQ84i7YEMY07qQ7hzKD4wN8ZHOJSVQ5XsP4Y=') \
# .config('spark.sql.catalog.opencatalog.warehouse','demo_catalog') \
# .config('spark.sql.catalog.opencatalog.scope','PRINCIPAL_ROLE:my_spark_admin_role') \
# .getOrCreate()


spark = SparkSession.builder.appName('iceberg_lab') \
.config('spark.jars.packages','org.apache.iceberg:iceberg-spark-runtime-3.5_2.12:1.4.1,software.amazon.awssdk:bundle:2.20.160,software.amazon.awssdk:url-connection-client:2.20.160') \
.config('spark.sql.extensions', 'org.apache.iceberg.spark.extensions.IcebergSparkSessionExtensions') \
.config('spark.sql.defaultCatalog', 'opencatalog') \
.config('spark.sql.catalog.opencatalog', 'org.apache.iceberg.spark.SparkCatalog').config('spark.sql.catalog.opencatalog.type', 'rest') \
.config('spark.sql.catalog.opencatalog.header.X-Iceberg-Access-Delegation', 'vended-credentials').config('spark.sql.catalog.opencatalog.uri','https://wv37264.us-east-2.aws.snowflakecomputing.com/polaris/api/catalog') \
.config('spark.sql.catalog.opencatalog.credential','klc4/u9+TVOqsZKCHcoin0HgSLw=:CI4ktHhqQ84i7YEMY07qQ7hzKD4wN8ZHOJSVQ5XsP4Y=') \
.config('spark.sql.catalog.opencatalog.warehouse', 'demo_catalog') \
.config('spark.sql.catalog.opencatalog.scope', 'PRINCIPAL_ROLE:my_spark_admin_role') \
.config("spark.driver.extraJavaOptions","-Dorg.apache.commons.logging.Log=org.apache.commons.logging.impl.SimpleLog -Dorg.apache.commons.logging.simplelog.log.org.apache.http=DEBUG") \
.config("spark.executor.extraJavaOptions","-Dorg.apache.commons.logging.Log=org.apache.commons.logging.impl.SimpleLog -Dorg.apache.commons.logging.simplelog.log.org.apache.http=DEBUG") \
.getOrCreate()

#Show namespaces
spark.sql("show namespaces").show()

#Create namespace
spark.sql("create namespace spark_demo")

#Use namespace
spark.sql("use namespace spark_demo")

#Show tables; this will show no tables since it is a new namespace
spark.sql("show tables").show()

#create a test table
spark.sql("create table test_table (col1 int) using iceberg");

#insert a record in the table
spark.sql("insert into test_table values (1)");

#query the table
spark.sql("select * from test_table").show();