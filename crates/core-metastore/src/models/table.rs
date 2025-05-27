use crate::error::{MetastoreError, MetastoreResult};
use iceberg_rust::{
    catalog::commit::{TableRequirement, TableUpdate as IcebergTableUpdate},
    spec::table_metadata::TableMetadata,
};
use iceberg_rust_spec::{
    partition::PartitionSpec, schema::Schema, sort::SortOrder, spec::identifier::Identifier,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use utoipa::ToSchema;
use validator::Validate;

use super::{SchemaIdent, VolumeIdent};

#[derive(Validate, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
/// A table identifier
pub struct TableIdent {
    #[validate(length(min = 1))]
    /// The name of the table
    pub table: String,
    #[validate(length(min = 1))]
    /// The schema the table belongs to
    pub schema: String,
    #[validate(length(min = 1))]
    /// The database the table belongs to
    pub database: String,
}

impl TableIdent {
    #[must_use]
    pub fn new(database: &str, schema: &str, table: &str) -> Self {
        Self {
            table: table.to_string(),
            schema: schema.to_string(),
            database: database.to_string(),
        }
    }
    #[must_use]
    pub fn to_iceberg_ident(&self) -> Identifier {
        let namespace = vec![self.schema.clone()];
        Identifier::new(&namespace, &self.table)
    }
}

impl From<TableIdent> for SchemaIdent {
    fn from(ident: TableIdent) -> Self {
        Self {
            database: ident.database,
            schema: ident.schema,
        }
    }
}

impl Display for TableIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.database, self.schema, self.table)
    }
}

#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, Eq, utoipa::ToSchema, strum::EnumString,
)]
#[serde(rename_all = "kebab-case")]
pub enum TableFormat {
    /*
    Avro,
    Orc,
    Delta,
    Json,
    Csv,*/
    Parquet,
    Iceberg,
}

impl Display for TableFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Parquet => "parquet".to_string(),
            Self::Iceberg => "iceberg".to_string(),
        };
        write!(f, "{str}")
    }
}

impl From<String> for TableFormat {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "parquet" => Self::Parquet,
            _ => Self::Iceberg,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Table {
    pub ident: TableIdent,
    pub metadata: TableMetadata,
    pub metadata_location: String,
    pub properties: HashMap<String, String>,
    pub volume_ident: Option<VolumeIdent>,
    pub volume_location: Option<String>,
    pub is_temporary: bool,
    pub format: TableFormat,
}

#[derive(Validate, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TableCreateRequest {
    #[validate(nested)]
    pub ident: TableIdent,
    pub properties: Option<HashMap<String, String>>,
    pub format: Option<TableFormat>,

    pub location: Option<String>,
    //pub schema: SimpleOrIcebergSchema,
    pub schema: Schema,
    pub partition_spec: Option<PartitionSpec>,
    pub sort_order: Option<SortOrder>,
    pub stage_create: Option<bool>,
    pub volume_ident: Option<VolumeIdent>,
    pub is_temporary: Option<bool>,
}

#[derive(ToSchema, Deserialize, Serialize)]
enum MyPrimitive {
    Int,
    Str,
    Decimal { precision: u32, scale: u32 },
}

#[derive(ToSchema, Deserialize, Serialize)]
struct MyMap {
    key: Box<TypeEnum>,
    value: Box<TypeEnum>,
}

#[derive(ToSchema, Deserialize, Serialize)]
struct MyList {
    element: Box<TypeEnum>,
}

#[derive(ToSchema, Deserialize, Serialize)]
struct MyStruct {
    fields: Vec<MyStructField>,
}

#[derive(ToSchema, Deserialize, Serialize)]
struct MyStructField {
    #[serde(rename = "type")]
    field_type: TypeEnum,
}

#[derive(ToSchema, Deserialize, Serialize)]
enum TypeEnum {
    Struct(MyStruct),
    List(MyList),
    Map(MyMap),
    Primitive(MyPrimitive),
}

#[derive(ToSchema, Deserialize, Serialize)]
struct MySchema {
    #[serde(flatten)]
    fields: MyStruct,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Config {
    pub defaults: HashMap<String, String>,
    pub overrides: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TableUpdate {
    /// Commit will fail if the requirements are not met.
    pub requirements: Vec<TableRequirement>,
    /// The updates of the table.
    pub updates: Vec<IcebergTableUpdate>,
}

pub struct TableRequirementExt(TableRequirement);

impl From<TableRequirement> for TableRequirementExt {
    fn from(requirement: TableRequirement) -> Self {
        Self(requirement)
    }
}

impl TableRequirementExt {
    #[must_use]
    pub const fn new(requirement: TableRequirement) -> Self {
        Self(requirement)
    }

    #[must_use]
    pub const fn inner(&self) -> &TableRequirement {
        &self.0
    }

    pub fn assert(&self, metadata: &TableMetadata, exists: bool) -> MetastoreResult<()> {
        match self.inner() {
            TableRequirement::AssertCreate => {
                if exists {
                    return Err(Box::new(MetastoreError::TableDataExists {
                        location: metadata.location.to_string(),
                    }));
                }
            }
            TableRequirement::AssertTableUuid { uuid } => {
                if &metadata.table_uuid != uuid {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table uuid does not match".to_string(),
                    }));
                }
            }
            TableRequirement::AssertCurrentSchemaId { current_schema_id } => {
                if metadata.current_schema_id != *current_schema_id {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table current schema id does not match".to_string(),
                    }));
                }
            }
            TableRequirement::AssertDefaultSortOrderId {
                default_sort_order_id,
            } => {
                if metadata.default_sort_order_id != *default_sort_order_id {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table default sort order id does not match".to_string(),
                    }));
                }
            }
            TableRequirement::AssertRefSnapshotId { r#ref, snapshot_id } => {
                if let Some(snapshot_id) = snapshot_id {
                    let snapshot_ref = metadata.refs.get(r#ref).ok_or_else(|| {
                        Box::new(MetastoreError::TableRequirementFailed {
                            message: "Table ref not found".to_string(),
                        })
                    })?;
                    if snapshot_ref.snapshot_id != *snapshot_id {
                        return Err(Box::new(MetastoreError::TableRequirementFailed {
                            message: "Table ref snapshot id does not match".to_string(),
                        }));
                    }
                } else if metadata.refs.contains_key(r#ref) {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table ref already exists".to_string(),
                    }));
                }
            }
            TableRequirement::AssertDefaultSpecId { default_spec_id } => {
                // ToDo: Harmonize the types of default_spec_id
                if metadata.default_spec_id != *default_spec_id {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table default spec id does not match".to_string(),
                    }));
                }
            }
            TableRequirement::AssertLastAssignedPartitionId {
                last_assigned_partition_id,
            } => {
                if metadata.last_partition_id != *last_assigned_partition_id {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table last assigned partition id does not match".to_string(),
                    }));
                }
            }
            TableRequirement::AssertLastAssignedFieldId {
                last_assigned_field_id,
            } => {
                if &metadata.last_column_id != last_assigned_field_id {
                    return Err(Box::new(MetastoreError::TableRequirementFailed {
                        message: "Table last assigned field id does not match".to_string(),
                    }));
                }
            }
        }
        Ok(())
    }
}
