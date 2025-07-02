use std::any::Any;
use std::sync::{Arc, OnceLock};

use crate::errors;
use crate::geospatial::data_types::{any_single_geometry_type_input, parse_to_native_array};
use crate::geospatial::error as geo_error;
use datafusion::arrow::datatypes::DataType;
use datafusion::logical_expr::scalar_doc_sections::DOC_SECTION_OTHER;
use datafusion::logical_expr::{ColumnarValue, Documentation, ScalarUDFImpl, Signature};
use datafusion_common::Result;
use datafusion_expr::ScalarFunctionArgs;
use geoarrow::algorithm::geo::ChamberlainDuquetteArea;
use snafu::ResultExt;

#[derive(Debug)]
pub struct Area {
    signature: Signature,
}

impl Area {
    pub fn new() -> Self {
        Self {
            signature: any_single_geometry_type_input(),
        }
    }
}

static DOCUMENTATION: OnceLock<Documentation> = OnceLock::new();

impl ScalarUDFImpl for Area {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "st_area"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Float64)
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        area(&args.args)
    }

    fn documentation(&self) -> Option<&Documentation> {
        Some(DOCUMENTATION.get_or_init(|| {
            Documentation::builder(
                DOC_SECTION_OTHER,
                "Returns the area of a geometry in square meters",
                "ST_Area(geom)",
            )
            .with_argument("geom", "geometry")
            .build()
        }))
    }
}

fn area(args: &[ColumnarValue]) -> Result<ColumnarValue> {
    let array = ColumnarValue::values_to_arrays(args)?
        .into_iter()
        .next()
        .ok_or_else(|| errors::ExpectedOnlyOneArgumentInSTAreaSnafu.build())?;
    let native_array = parse_to_native_array(&array)?;
    let area = native_array
        .as_ref()
        .chamberlain_duquette_unsigned_area()
        .context(geo_error::GeoArrowSnafu)?;
    Ok(ColumnarValue::Array(Arc::new(area)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::ArrayRef;
    use datafusion::arrow::array::cast::AsArray;
    use datafusion::arrow::array::types::Float64Type;
    use datafusion::logical_expr::ColumnarValue;
    use geo_types::{line_string, point, polygon};
    use geoarrow::ArrayBase;
    use geoarrow::array::LineStringBuilder;
    use geoarrow::array::{CoordType, PointBuilder, PolygonBuilder};
    use geoarrow::datatypes::Dimension;

    #[test]
    #[allow(clippy::unwrap_used, clippy::float_cmp)]
    fn test_area() {
        let dim = Dimension::XY;
        let ct = CoordType::Separated;

        let args: [(ArrayRef, [f64; 2]); 3] = [
            (
                {
                    let data = vec![
                        line_string![(x: 1., y: 1.), (x: 1., y: 2.), (x: 1., y: 1.), (x: 0., y: 1.), (x: 0., y: 0.)],
                        line_string![(x: 2., y: 2.), (x: 3., y: 2.), (x: 3., y: 3.), (x: 2., y: 3.), (x: 2., y: 2.)],
                    ];
                    let array =
                        LineStringBuilder::from_line_strings(&data, dim, ct, Arc::default())
                            .finish();
                    array.to_array_ref()
                },
                [0., 0.],
            ),
            (
                {
                    let data = [point! {x: 0., y: 0.}, point! {x: 0., y: 0.}];
                    let array =
                        PointBuilder::from_points(data.iter(), dim, ct, Arc::default()).finish();
                    array.to_array_ref()
                },
                [0., 0.],
            ),
            (
                {
                    let data = vec![
                        polygon![(x: 0., y: 0.), (x: 0., y: 1.0), (x: 1., y: 2.), (x: 2., y: 0.), (x:0., y:0.)],
                        polygon![(x: 3.3, y: 30.4), (x: 1.7, y: 24.6), (x: 13.4, y: 25.1), (x: 14.4, y: 31.0),(x:3.3,y:30.4)],
                    ];
                    let array =
                        PolygonBuilder::from_polygons(&data, dim, ct, Arc::default()).finish();
                    array.to_array_ref()
                },
                [30_974_725_215., 723_055_529_500.],
            ),
        ];

        for (arr, exp) in args {
            let args = ScalarFunctionArgs {
                args: vec![ColumnarValue::Array(arr)],
                number_rows: 2,
                return_type: &DataType::Null,
            };
            let area_fn = Area::new();
            let result = area_fn.invoke_with_args(args).unwrap().to_array(2).unwrap();
            let result = result.as_primitive::<Float64Type>();
            assert_eq!(result.value(0).round(), exp[0]);
            assert_eq!(result.value(1).round(), exp[1]);
        }
    }
}
