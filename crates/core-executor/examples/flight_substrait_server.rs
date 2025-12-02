use arrow::ipc::writer::{DictionaryTracker, IpcDataGenerator};
use arrow_flight::utils::flight_data_to_arrow_batch;
use arrow_flight::{
    Action, ActionType, Criteria, Empty, FlightData, FlightDescriptor, FlightInfo,
    HandshakeRequest, HandshakeResponse, PutResult, SchemaResult, Ticket,
};
use arrow_flight::{PollInfo, SchemaAsIpc};
use arrow_flight::{
    flight_service_server::FlightService, flight_service_server::FlightServiceServer,
};
use datafusion::arrow::datatypes::Schema;
use datafusion::arrow::error::ArrowError;
use datafusion::catalog::CatalogProvider;
use datafusion::catalog::SchemaProvider;
use datafusion::catalog::TableProvider;
use datafusion::catalog::memory::{MemoryCatalogProvider, MemorySchemaProvider};
use datafusion::prelude::*;
use datafusion_substrait::logical_plan::consumer::from_substrait_plan;
use futures::stream::BoxStream;
use prost::Message as _;
use substrait::proto::Plan;
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

#[derive(Clone)]
pub struct FlightServiceImpl {
    temp_schema: std::sync::Arc<MemorySchemaProvider>,
}

#[tonic::async_trait]
impl FlightService for FlightServiceImpl {
    type HandshakeStream = BoxStream<'static, Result<HandshakeResponse, Status>>;
    type ListFlightsStream = BoxStream<'static, Result<FlightInfo, Status>>;
    type DoGetStream = BoxStream<'static, Result<FlightData, Status>>;
    type DoPutStream = BoxStream<'static, Result<PutResult, Status>>;
    type DoActionStream = BoxStream<'static, Result<arrow_flight::Result, Status>>;
    type ListActionsStream = BoxStream<'static, Result<ActionType, Status>>;
    type DoExchangeStream = BoxStream<'static, Result<FlightData, Status>>;

    async fn get_schema(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<SchemaResult>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn do_get(
        &self,
        request: Request<Ticket>,
    ) -> Result<Response<Self::DoGetStream>, Status> {
        let ticket = request.into_inner();
        // Expect Substrait Plan in bytes
        let plan =
            Plan::decode(ticket.ticket).map_err(|e| Status::invalid_argument(e.to_string()))?;

        // Build SessionContext and convert Substrait -> LogicalPlan
        let ctx = SessionContext::new();
        // Register in-memory catalogs for pushed tables
        let catalog_push = std::sync::Arc::new(MemoryCatalogProvider::new());
        catalog_push
            .register_schema("temp", self.temp_schema.clone())
            .map_err(to_tonic_err)?;
        ctx.register_catalog("push", catalog_push);
        // Also register benchmark_database.benchmark_schema mapped to same temp schema
        let catalog_bench = std::sync::Arc::new(MemoryCatalogProvider::new());
        catalog_bench
            .register_schema("benchmark_schema", self.temp_schema.clone())
            .map_err(to_tonic_err)?;
        ctx.register_catalog("benchmark_database", catalog_bench);
        let logical_plan = from_substrait_plan(&ctx.state(), &plan)
            .await
            .map_err(to_tonic_err)?;
        let df = DataFrame::new(ctx.state(), logical_plan);

        let schema = df.schema().clone().into();
        let results = df.collect().await.map_err(to_tonic_err)?;
        if results.is_empty() {
            return Err(Status::internal("There were no results from ticket"));
        }
        println!("Results: {:?}", results);

        let options = arrow::ipc::writer::IpcWriteOptions::default();
        let schema_flight_data = SchemaAsIpc::new(&schema, &options);

        let mut flights = vec![FlightData::from(schema_flight_data)];

        let encoder = IpcDataGenerator::default();
        let mut tracker = DictionaryTracker::new(false);
        for batch in &results {
            let (flight_dictionaries, flight_batch) = encoder
                .encoded_batch(batch, &mut tracker, &options)
                .map_err(|e: ArrowError| Status::internal(e.to_string()))?;
            flights.extend(flight_dictionaries.into_iter().map(Into::into));
            flights.push(flight_batch.into());
        }

        let output = futures::stream::iter(flights.into_iter().map(Ok));
        Ok(Response::new(Box::pin(output) as Self::DoGetStream))
    }

    async fn handshake(
        &self,
        _request: Request<Streaming<HandshakeRequest>>,
    ) -> Result<Response<Self::HandshakeStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn list_flights(
        &self,
        _request: Request<Criteria>,
    ) -> Result<Response<Self::ListFlightsStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<FlightInfo>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn do_put(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoPutStream>, Status> {
        let mut stream = _request.into_inner();
        // First message should be schema
        let first = stream
            .message()
            .await?
            .ok_or_else(|| Status::invalid_argument("empty DoPut stream"))?;
        let schema = std::sync::Arc::new(
            Schema::try_from(&first).map_err(|e| Status::invalid_argument(e.to_string()))?,
        );

        // Use descriptor.path to get table name: ["push", "temp", "<name>"] or ["<name>"]
        let table_name = if let Some(desc) = &first.flight_descriptor {
            if !desc.path.is_empty() {
                desc.path
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "temp".to_string())
            } else {
                "temp".to_string()
            }
        } else {
            "temp".to_string()
        };

        let mut batches = Vec::new();
        let dicts = std::collections::HashMap::new();
        while let Some(fd) = stream.message().await? {
            if fd.data_body.is_empty() && fd.data_header.is_empty() {
                continue;
            }
            match flight_data_to_arrow_batch(&fd, schema.clone(), &dicts) {
                Ok(batch) => batches.push(batch),
                Err(_) => {}
            }
        }
        let table = datafusion::datasource::MemTable::try_new(schema.clone(), vec![batches])
            .map_err(to_tonic_err)?;
        self.temp_schema
            .register_table(
                table_name,
                std::sync::Arc::new(table) as std::sync::Arc<dyn TableProvider>,
            )
            .map_err(to_tonic_err)?;
        let output = futures::stream::empty();
        println!("Output: {:?}", output);
        Ok(Response::new(Box::pin(output) as Self::DoPutStream))
    }

    async fn do_action(
        &self,
        _request: Request<Action>,
    ) -> Result<Response<Self::DoActionStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn list_actions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::ListActionsStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn do_exchange(
        &self,
        _request: Request<Streaming<FlightData>>,
    ) -> Result<Response<Self::DoExchangeStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn poll_flight_info(
        &self,
        _request: Request<FlightDescriptor>,
    ) -> Result<Response<PollInfo>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }
}

fn to_tonic_err(e: datafusion::error::DataFusionError) -> Status {
    Status::internal(format!("{e:?}"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let service = FlightServiceImpl {
        temp_schema: std::sync::Arc::new(MemorySchemaProvider::new()),
    };
    let svc = FlightServiceServer::new(service);
    println!("Substrait Flight server listening on {addr:?}");
    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}
