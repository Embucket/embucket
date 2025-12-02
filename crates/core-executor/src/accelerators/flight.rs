use super::{AcceleratorKind, ExternalAccelerator};
use arrow::ipc::writer::{DictionaryTracker, IpcDataGenerator, IpcWriteOptions};
use arrow_flight::SchemaAsIpc;
use arrow_flight::flight_service_client::FlightServiceClient;
use arrow_flight::{FlightData, FlightDescriptor, Ticket};
use datafusion::execution::SessionState;
use datafusion::logical_expr::LogicalPlan;
use datafusion_substrait::logical_plan::producer::to_substrait_plan;
use futures::StreamExt;
use prost::Message as _;
use snafu::ResultExt;
use std::sync::Arc;
use tonic::transport::Channel;

/// Skeleton Arrow Flight Substrait client.
/// Implementation will submit Substrait plans to a remote engine (Acero/Velox)
pub struct FlightSubstraitClient {
    pub endpoint: String,
    pub kind: AcceleratorKind,
}

impl FlightSubstraitClient {
    #[must_use]
    pub fn new(endpoint: String, kind: AcceleratorKind) -> Self {
        Self { endpoint, kind }
    }

    pub async fn open(&self) -> crate::Result<FlightServiceClient<Channel>> {
        let client = FlightServiceClient::connect(self.endpoint.clone())
            .await
            .map_err(|e| datafusion_common::DataFusionError::Execution(e.to_string()))
            .context(crate::error::DataFusionSnafu)?;
        Ok(client)
    }
}

#[async_trait::async_trait]
impl ExternalAccelerator for FlightSubstraitClient {
    fn kind(&self) -> AcceleratorKind {
        self.kind
    }

    async fn execute(
        &self,
        plan: &LogicalPlan,
        state: &SessionState,
    ) -> crate::Result<datafusion::execution::SendableRecordBatchStream> {
        // Serialize plan to Substrait bytes
        let substrait = to_substrait_plan(plan, state).context(crate::error::DataFusionSnafu)?;
        let substrait_bytes = substrait.encode_to_vec();

        // Connect and issue do_get with substrait bytes as ticket (server-defined)
        let mut client: FlightServiceClient<Channel> =
            FlightServiceClient::connect(self.endpoint.clone())
                .await
                .map_err(|e| datafusion_common::DataFusionError::Execution(e.to_string()))
                .context(crate::error::DataFusionSnafu)?;

        let request = tonic::Request::new(Ticket {
            ticket: bytes::Bytes::from(substrait_bytes),
        });
        let mut stream = client
            .do_get(request)
            .await
            .map_err(|e| datafusion_common::DataFusionError::Execution(e.to_string()))
            .context(crate::error::DataFusionSnafu)?
            .into_inner();

        // Convert Flight stream to Arrow RecordBatch stream
        use arrow_flight::utils::flight_data_to_arrow_batch;
        use datafusion::arrow::datatypes::Schema;
        use datafusion::arrow::record_batch::RecordBatch;
        use datafusion::execution::SendableRecordBatchStream;
        use datafusion_common::DataFusionError;
        use datafusion_physical_plan::stream::RecordBatchStreamAdapter;
        use std::collections::HashMap;

        // Read schema
        let first = stream
            .message()
            .await
            .map_err(|e| DataFusionError::Execution(e.to_string()))
            .context(crate::error::DataFusionSnafu)?
            .ok_or_else(|| DataFusionError::Execution("empty Flight stream".into()))
            .context(crate::error::DataFusionSnafu)?;
        let schema = Arc::new(
            Schema::try_from(&first)
                .map_err(|e| DataFusionError::Execution(e.to_string()))
                .context(crate::error::DataFusionSnafu)?,
        );
        let dictionaries_by_field = HashMap::new();

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<RecordBatch, DataFusionError>>(2);
        tokio::spawn({
            let schema = schema.clone();
            async move {
                while let Ok(Some(flight_data)) = stream
                    .message()
                    .await
                    .map_err(|e| DataFusionError::Execution(e.to_string()))
                {
                    match flight_data_to_arrow_batch(
                        &flight_data,
                        schema.clone(),
                        &dictionaries_by_field,
                    ) {
                        Ok(batch) => {
                            if tx.send(Ok(batch)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(Err(DataFusionError::Execution(e.to_string())))
                                .await;
                            break;
                        }
                    }
                }
            }
        });

        let s =
            tokio_stream::wrappers::ReceiverStream::new(rx).map(|res| res.map_err(|e| e.into()));
        let stream: SendableRecordBatchStream =
            Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(s)));
        Ok(stream)
    }
}

impl FlightSubstraitClient {
    pub async fn execute_with_client(
        &self,
        client: &mut FlightServiceClient<Channel>,
        plan: &LogicalPlan,
        state: &SessionState,
    ) -> crate::Result<datafusion::execution::SendableRecordBatchStream> {
        let substrait = to_substrait_plan(plan, state).context(crate::error::DataFusionSnafu)?;
        let substrait_bytes = substrait.encode_to_vec();
        let request = tonic::Request::new(Ticket {
            ticket: bytes::Bytes::from(substrait_bytes),
        });
        let mut stream = client
            .do_get(request)
            .await
            .map_err(|e| datafusion_common::DataFusionError::Execution(e.to_string()))
            .context(crate::error::DataFusionSnafu)?
            .into_inner();

        use arrow_flight::utils::flight_data_to_arrow_batch;
        use datafusion::arrow::datatypes::Schema;
        use datafusion::arrow::record_batch::RecordBatch;
        use datafusion::execution::SendableRecordBatchStream;
        use datafusion_common::DataFusionError;
        use datafusion_physical_plan::stream::RecordBatchStreamAdapter;
        use std::collections::HashMap;

        let first = stream
            .message()
            .await
            .map_err(|e| DataFusionError::Execution(e.to_string()))
            .context(crate::error::DataFusionSnafu)?
            .ok_or_else(|| DataFusionError::Execution("empty Flight stream".into()))
            .context(crate::error::DataFusionSnafu)?;
        let schema = Arc::new(
            Schema::try_from(&first)
                .map_err(|e| DataFusionError::Execution(e.to_string()))
                .context(crate::error::DataFusionSnafu)?,
        );
        let dictionaries_by_field = HashMap::new();

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<RecordBatch, DataFusionError>>(2);
        tokio::spawn({
            let schema = schema.clone();
            async move {
                while let Ok(Some(flight_data)) = stream
                    .message()
                    .await
                    .map_err(|e| DataFusionError::Execution(e.to_string()))
                {
                    match flight_data_to_arrow_batch(
                        &flight_data,
                        schema.clone(),
                        &dictionaries_by_field,
                    ) {
                        Ok(batch) => {
                            if tx.send(Ok(batch)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx
                                .send(Err(DataFusionError::Execution(e.to_string())))
                                .await;
                            break;
                        }
                    }
                }
            }
        });

        let s =
            tokio_stream::wrappers::ReceiverStream::new(rx).map(|res| res.map_err(|e| e.into()));
        let stream: SendableRecordBatchStream =
            Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(s)));
        Ok(stream)
    }

    pub async fn put_table(
        &self,
        client: &mut FlightServiceClient<Channel>,
        table_name: &str,
        schema: Arc<datafusion::arrow::datatypes::Schema>,
        mut batches: Vec<datafusion::arrow::record_batch::RecordBatch>,
    ) -> crate::Result<()> {
        // Build FlightDescriptor path = ["push","temp", table_name]
        let descriptor = FlightDescriptor {
            r#type: 0,
            cmd: bytes::Bytes::new(),
            path: vec![
                "push".to_string(),
                "temp".to_string(),
                table_name.to_string(),
            ],
        };

        // Prepare schema FlightData
        let options = IpcWriteOptions::default();
        let schema_fd = SchemaAsIpc::new(schema.as_ref(), &options);
        let mut flights: Vec<FlightData> =
            vec![FlightData::from(schema_fd).with_descriptor(descriptor.clone())];

        // Encode batches
        let encoder = IpcDataGenerator::default();
        let mut tracker = DictionaryTracker::new(false);
        for batch in &mut batches {
            let (dicts, b_fd) = encoder
                .encoded_batch(batch, &mut tracker, &options)
                .map_err(|e| datafusion_common::DataFusionError::Execution(e.to_string()))
                .context(crate::error::DataFusionSnafu)?;
            flights.extend(dicts.into_iter().map(Into::into));
            flights.push(b_fd.into());
        }

        if flights.len() > 1 {
            flights[1..]
                .iter_mut()
                .for_each(|fd| fd.flight_descriptor = Some(descriptor.clone()));
        }

        let stream = futures::stream::iter(flights);
        client
            .do_put(stream)
            .await
            .map_err(|e| datafusion_common::DataFusionError::Execution(e.to_string()))
            .context(crate::error::DataFusionSnafu)?;
        Ok(())
    }
}
