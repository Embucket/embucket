use crate::datetime::timestamp_from_parts::to_primitive_array;
use crate::errors;
use crate::geospatial::data_types::{
    BOX2D_TYPE, BOX3D_TYPE, GEOMETRY_TYPE, LINE_STRING_TYPE, POINT2D_TYPE, POINT3D_TYPE,
    POLYGON_2D_TYPE, parse_to_native_array,
};
use crate::geospatial::error as geo_error;
use datafusion::arrow::array::types::Int64Type;
use datafusion::arrow::datatypes::DataType;
use datafusion::logical_expr::scalar_doc_sections::DOC_SECTION_OTHER;
use datafusion::logical_expr::{
    ColumnarValue, Documentation, ScalarUDFImpl, Signature, TypeSignature, Volatility,
};
use datafusion_common::Result;
use datafusion_expr::ScalarFunctionArgs;
use geo_traits::LineStringTrait;
use geoarrow::ArrayBase;
use geoarrow::array::{AsNativeArray, PointBuilder};
use geoarrow::error::GeoArrowError;
use geoarrow::trait_::ArrayAccessor;
use geoarrow_schema::{CoordType, Dimension};
use snafu::ResultExt;
use std::any::Any;
use std::error::Error;
use std::sync::{Arc, OnceLock};

static DOCUMENTATION: OnceLock<Documentation> = OnceLock::new();

macro_rules! create_line_string_udf {
    ($name:ident, $func_name:expr, $index:expr, $doc:expr, $syntax:expr) => {
        #[derive(Debug)]
        pub struct $name {
            signature: Signature,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    signature: Signature::exact(
                        vec![LINE_STRING_TYPE.into()],
                        Volatility::Immutable,
                    ),
                }
            }
        }

        impl ScalarUDFImpl for $name {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn name(&self) -> &'static str {
                $func_name
            }

            fn signature(&self) -> &Signature {
                &self.signature
            }

            fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
                Ok(POINT2D_TYPE.into())
            }

            fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
                get_n_point(&args.args, $index)
            }

            fn documentation(&self) -> Option<&Documentation> {
                Some(DOCUMENTATION.get_or_init(|| {
                    Documentation::builder(DOC_SECTION_OTHER, $doc, $syntax)
                        .with_argument("g1", "geometry")
                        .with_related_udf("st_startpoint")
                        .with_related_udf("st_pointn")
                        .with_related_udf("st_endpoint")
                        .build()
                }))
            }
        }
    };
}

create_line_string_udf!(
    EndPoint,
    "st_endpoint",
    None,
    "Returns the last point of a LINESTRING geometry as a POINT. Returns NULL if the input is not a LINESTRING",
    "ST_EndPoint(line_string)"
);

create_line_string_udf!(
    StartPoint,
    "st_startpoint",
    Some(1),
    "Returns the first point of a LINESTRING geometry as a POINT.",
    "ST_StartPoint(geom)"
);

#[derive(Debug)]
pub struct PointN {
    signature: Signature,
}

impl PointN {
    pub fn new() -> Self {
        Self {
            signature: Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![POINT2D_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![POINT3D_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![BOX2D_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![BOX3D_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![LINE_STRING_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![POLYGON_2D_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![POINT2D_TYPE.into(), DataType::Int64]),
                    TypeSignature::Exact(vec![GEOMETRY_TYPE.into(), DataType::Int64]),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl ScalarUDFImpl for PointN {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &'static str {
        "st_pointn"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(POINT2D_TYPE.into())
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let args = args.args;
        if args.len() < 2 {
            return errors::ExpectedTwoArgumentsInSTPointNSnafu.fail()?;
        }
        let index = to_primitive_array::<Int64Type>(&args[1])?.value(0);
        get_n_point(&args, Some(index))
    }

    fn documentation(&self) -> Option<&Documentation> {
        Some(DOCUMENTATION.get_or_init(|| {
            Documentation::builder(
                DOC_SECTION_OTHER,
                "Returns a Point at a specified index in a LineString. Returns NULL if the input is not a LINESTRING",
                "ST_PointN(line_string)")
                .with_argument("g1", "geometry")
                .build()
        }))
    }
}

fn get_n_point(args: &[ColumnarValue], n: Option<i64>) -> Result<ColumnarValue> {
    let array = ColumnarValue::values_to_arrays(args)?
        .into_iter()
        .next()
        .ok_or_else(|| errors::ExpectedAtLeastOneArgumentSnafu.build())?;

    let native_array = parse_to_native_array(&array)?;

    let native_array_ref = native_array.as_ref();
    let line_string_array = native_array_ref
        .as_line_string_opt()
        .ok_or_else(|| errors::ExpectedGeometryTypedArraySnafu.build())?;

    let mut output_builder = PointBuilder::with_capacity_and_options(
        Dimension::XY,
        line_string_array.len(),
        CoordType::Separated,
        Arc::default(),
    );

    for line in line_string_array.iter() {
        if let Some(line_string) = line {
            let pos = if let Some(n) = n {
                let index = if n < 0 {
                    line_string.num_coords().try_into().unwrap_or(0) + n
                } else {
                    n - 1
                };
                index
                    .try_into()
                    .map_err(|_| errors::IndexOutOfBoundsSnafu.build())?
            } else {
                line_string.num_coords() - 1
            };
            output_builder.push_coord(line_string.coord(pos).as_ref());
        } else {
            output_builder.push_null();
        }
    }

    Ok(output_builder.finish().into_array_ref().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{EndPoint, StartPoint};
    use datafusion::arrow::array::Array;
    use datafusion::logical_expr::ColumnarValue;
    use geo_types::line_string;
    use geoarrow::array::{LineStringBuilder, PointArray};
    use geoarrow::datatypes::Dimension;
    use geoarrow::trait_::ArrayAccessor;
    use geozero::ToWkt;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_start_end_point() {
        let data = vec![
            line_string![(x: 1., y: 1.), (x: 1., y: 0.), (x: 1., y: 1.)],
            line_string![(x: 2., y: 2.), (x: 3., y: 2.), (x: 3., y: 3.)],
            line_string![(x: 2., y: 2.), (x: 3., y: 2.)],
        ];
        let array = LineStringBuilder::from_line_strings(
            &data,
            Dimension::XY,
            CoordType::Separated,
            Arc::default(),
        )
        .finish()
        .to_array_ref();

        let udfs: Vec<Box<dyn ScalarUDFImpl>> =
            vec![Box::new(StartPoint::new()), Box::new(EndPoint::new())];
        let results: [[&str; 3]; 2] = [
            ["POINT(1 1)", "POINT(2 2)", "POINT(2 2)"],
            ["POINT(1 1)", "POINT(3 3)", "POINT(3 2)"],
        ];

        for (idx, udf) in udfs.iter().enumerate() {
            let result = udf
                .invoke_with_args(ScalarFunctionArgs {
                    args: vec![ColumnarValue::Array(array.clone())],
                    number_rows: 3,
                    return_type: &DataType::Null,
                })
                .unwrap();
            let result = result.to_array(3).unwrap();
            assert_eq!(result.data_type(), &POINT2D_TYPE.into());
            let result = PointArray::try_from((result.as_ref(), Dimension::XY)).unwrap();
            assert_eq!(result.get(0).unwrap().to_wkt().unwrap(), results[idx][0]);
            assert_eq!(result.get(1).unwrap().to_wkt().unwrap(), results[idx][1]);
            assert_eq!(result.get(2).unwrap().to_wkt().unwrap(), results[idx][2]);
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_point_n() {
        let data = vec![
            line_string![(x: 0., y: 0.), (x: 1., y: 0.), (x: 1., y: 1.), (x: 4.1, y: 4.1)],
            line_string![(x: 2., y: 2.), (x: 3., y: 2.), (x: 3., y: 3.)],
            line_string![(x: 2., y: 2.), (x: 4., y: 2.)],
        ];
        let array = LineStringBuilder::from_line_strings(
            &data,
            Dimension::XY,
            CoordType::Separated,
            Arc::default(),
        )
        .finish();

        let cases: [(i64, bool, [&str; 3]); 5] = [
            (1, true, ["POINT(0 0)", "POINT(2 2)", "POINT(2 2)"]),
            (2, true, ["POINT(1 0)", "POINT(3 2)", "POINT(4 2)"]),
            (-1, true, ["POINT(4.1 4.1)", "POINT(3 3)", "POINT(4 2)"]),
            (-2, true, ["POINT(1 1)", "POINT(3 2)", "POINT(2 2)"]),
            (-10, false, ["", "", ""]),
        ];

        for (index, ok, exp) in cases {
            let data = array.to_array_ref();
            let args = ScalarFunctionArgs {
                args: vec![
                    ColumnarValue::Array(data),
                    ColumnarValue::Scalar(index.into()),
                ],
                number_rows: 3,
                return_type: &DataType::Null,
            };

            let point_n = PointN::new();
            let result = point_n.invoke_with_args(args);

            if ok {
                let result = result.unwrap().to_array(3).unwrap();
                assert_eq!(result.data_type(), &POINT2D_TYPE.into());
                let result = PointArray::try_from((result.as_ref(), Dimension::XY)).unwrap();
                assert_eq!(result.get(0).unwrap().to_wkt().unwrap(), exp[0]);
                assert_eq!(result.get(1).unwrap().to_wkt().unwrap(), exp[1]);
                assert_eq!(result.get(2).unwrap().to_wkt().unwrap(), exp[2]);
            } else {
                assert_eq!(
                    result.err().unwrap().to_string(),
                    "Execution error: Index out of bounds"
                );
            }
        }
    }
}
