# catalog-metastore

Core library responsible for the abstraction and interaction with the underlying metadata storage system. Defines data models and traits for metastore operations.

## Purpose

This crate provides a consistent way for other Embucket components to access and manipulate metadata about catalogs, schemas, tables, and other entities, abstracting the specific storage backend.

## Timeouts related Environment Variables

|Variable Name  |Default Value    |
|:--------------|:----------------|
|AWS_SDK_CONNECT_TIMEOUT_SECS|3|
|AWS_SDK_OPERATION_TIMEOUT_SECS|30|
|AWS_SDK_OPERATION_ATTEMPT_TIMEOUT_SECS|10|
|ICEBERG_CREATE_TABLE_TIMEOUT_SECS|30|
|ICEBERG_CATALOG_TIMEOUT_SECS|10|
|OBJECT_STORE_TIMEOUT_SECS|30|
|OBJECT_STORE_CONNECT_TIMEOUT_SECS|3|
