# catalog

Implements DataFusion's `CatalogProvider` and related traits, enabling the query engine to discover and interact with schemas, tables, and views managed by Embucket's metastore and external catalog sources like Iceberg.

## Purpose

This crate acts as a bridge between Embucket's metadata management (`catalog-metastore`) and the DataFusion query engine (`executor`), allowing DataFusion to understand the structure of data accessible via Embucket.

## Catalog Timeouts

Timeouts are applied to async functions of IcebergCatalog.

| Interface Function  | aws_sdk_timeout_config | generic |
|----------|----------|----------|
| create_namespace | ✓ | ✓ |
| drop_namespace | ✓ | ✓ |
| load_namespace |  |  |
| update_namespace |  |  |
| namespace_exists |  | ✓ |
| list_tabulars |  | ✓ |
| list_namespaces |  | ✓ |
| tabular_exists |  | ✓ |
| load_tabular |  |  |
| create_table | todo |  |
| update_table | todo |  |
| drop_table |  | ✓ |
