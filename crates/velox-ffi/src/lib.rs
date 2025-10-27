//! Minimal Velox FFI facade that links to a C/C++ shim for Velox.
use arrow::array::{make_array, ArrayRef};
use arrow_array::ffi::{FFI_ArrowArray as ArrowArrayFFI, from_ffi, export_array_into_raw};
use arrow_schema::ffi::FFI_ArrowSchema as ArrowSchemaFFI;
use arrow::record_batch::RecordBatch;
use arrow_schema::Schema;
// no Pin usage after iterator refactor
use std::sync::Arc;
use std::ffi::{c_char, c_int, c_longlong, c_uchar, c_void};

#[allow(non_camel_case_types)]
type int32_t = c_int;
#[allow(non_camel_case_types)]
type int64_t = c_longlong;

type VeloxSessionHandle = *mut c_void;
type VeloxStreamHandle = *mut c_void;

// C's _Bool is typically represented as u8 for FFI purposes
#[allow(non_camel_case_types)]
type c_bool = u8;

type VeloxCreateSession = unsafe extern "C" fn(err: *mut c_char, err_len: usize) -> VeloxSessionHandle;
type VeloxFreeSession = unsafe extern "C" fn(s: VeloxSessionHandle);
type VeloxRegisterTableArrow = unsafe extern "C" fn(
    s: VeloxSessionHandle,
    name: *const c_char,
    schema: *const ArrowSchemaFFI,
    columns: *const *const ArrowArrayFFI,
    num_cols: int32_t,
    num_rows: int64_t,
    err: *mut c_char,
    err_len: usize,
) -> c_bool;
type VeloxExecuteSubstrait = unsafe extern "C" fn(
    s: VeloxSessionHandle,
    plan: *const c_uchar,
    len: usize,
    err: *mut c_char,
    err_len: usize,
) -> VeloxStreamHandle;
type VeloxStreamNext = unsafe extern "C" fn(
    st: VeloxStreamHandle,
    out_schema: *mut ArrowSchemaFFI,
    out_batch: *mut ArrowArrayFFI,
    out_num_cols: *mut int32_t,
    out_num_rows: *mut int64_t,
    out_end: *mut c_bool,
    err: *mut c_char,
    err_len: usize,
) -> c_bool;
type VeloxStreamFree = unsafe extern "C" fn(st: VeloxStreamHandle);

#[derive(thiserror::Error, Debug)]
pub enum VeloxError {
    #[error("velox feature not compiled")] 
    FeatureDisabled,
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, VeloxError>;

#[derive(Clone, Debug, Default)]
pub struct VeloxConfig {
    pub threads: Option<usize>,
}

pub struct VeloxSession {
    _cfg: VeloxConfig,
    // Dynamic function pointers (None if library not present)
    lib: Option<Arc<VeloxDynLib>>,
    handle: Option<VeloxSessionHandle>,
}

impl VeloxSession {
    pub fn new(cfg: VeloxConfig) -> Result<Self> {
        let lib = VeloxDynLib::load().map(Arc::new);
        // Best-effort create a native session if library is present
        let handle = if let Some(ref l) = lib {
            // Allocate small error buffer
            let mut buf = vec![0i8; 256];
            let h = unsafe { (l.create)(buf.as_mut_ptr() as *mut c_char, buf.len()) };
            if h.is_null() {
                // Do not fail hard; report as feature disabled to caller paths that require it
                None
            } else {
                Some(h)
            }
        } else { None };
        Ok(Self { _cfg: cfg, lib, handle })
    }

    pub fn register_table_arrow(&mut self, name: &str, schema: &Schema, batches: &[RecordBatch]) -> Result<()> {
        let lib = self.lib.clone().ok_or(VeloxError::FeatureDisabled)?;
        let handle = self.handle.ok_or(VeloxError::FeatureDisabled)?;
        // Export schema once
        // Build a full FFI schema for the table
        let c_schema = Box::new(ArrowSchemaFFI::try_from(schema).map_err(|e| VeloxError::Message(format!("FFI_ArrowSchema::try_from: {e}")))?);
        let cname = std::ffi::CString::new(name).map_err(|e| VeloxError::Message(format!("bad name: {e}")))?;

        for batch in batches {
            let num_cols = batch.num_columns() as i32;
            let num_rows = batch.num_rows() as i64;

            // Export each column
            let mut col_boxes: Vec<Box<ArrowArrayFFI>> = Vec::with_capacity(batch.num_columns());
            let mut col_schema_boxes: Vec<Box<ArrowSchemaFFI>> = Vec::with_capacity(batch.num_columns());
            let mut col_ptrs: Vec<*const ArrowArrayFFI> = Vec::with_capacity(batch.num_columns());
            for i in 0..batch.num_columns() {
                let col: ArrayRef = batch.column(i).clone();
                let mut c_arr = std::mem::MaybeUninit::<ArrowArrayFFI>::uninit();
                let mut c_col_schema = std::mem::MaybeUninit::<ArrowSchemaFFI>::uninit();
                unsafe { export_array_into_raw(col.clone(), c_arr.as_mut_ptr(), c_col_schema.as_mut_ptr()) }
                    .map_err(|e| VeloxError::Message(format!("export_array_into_raw: {e}")))?;
                let boxed_arr = unsafe { Box::new(c_arr.assume_init()) };
                let boxed_schema = unsafe { Box::new(c_col_schema.assume_init()) };
                col_ptrs.push(boxed_arr.as_ref() as *const ArrowArrayFFI);
                col_boxes.push(boxed_arr);
                col_schema_boxes.push(boxed_schema);
            }

            // Call into native register
            let mut err = vec![0i8; 256];
            let ok = unsafe {
                (lib.register)(
                    handle,
                    cname.as_ptr(),
                    c_schema.as_ref() as *const ArrowSchemaFFI,
                    col_ptrs.as_ptr(),
                    num_cols,
                    num_rows,
                    err.as_mut_ptr() as *mut c_char,
                    err.len(),
                )
            };
            // Drop col_boxes and col_schema_boxes to run release callbacks now that registration call returned
            drop(col_boxes);
            drop(col_schema_boxes);
            if ok == 0 {
                return Err(VeloxError::Message("velox_register_table_arrow failed".into()));
            }
        }
        // c_schema will drop and run release
        Ok(())
    }

    pub fn execute_substrait_to_arrow_stream(
        &mut self,
        plan: &[u8],
    ) -> Result<VeloxArrowStream> {
        let lib = self.lib.clone().ok_or(VeloxError::FeatureDisabled)?;
        let handle = self.handle.ok_or(VeloxError::FeatureDisabled)?;
        // Attempt native execution. In stub builds, this produces an empty stream.
        let mut err = vec![0i8; 256];
        let stream_handle = unsafe {
            (lib.exec)(
                handle,
                plan.as_ptr(),
                plan.len(),
                err.as_mut_ptr() as *mut c_char,
                err.len(),
            )
        };
        if stream_handle.is_null() {
            return Err(VeloxError::Message("velox_execute_substrait failed".into()));
        }
        Ok(VeloxArrowStream::new(lib, stream_handle))
    }
}

pub struct VeloxArrowStream {
    pub schema: Arc<Schema>,
    pub batches: Box<dyn Iterator<Item = RecordBatch> + Send + 'static>, 
    // Keep native resources alive for the duration of the iterator
    _native: Option<VeloxStreamNative>,
}

impl VeloxArrowStream {
    pub fn into_batches(self) -> (Arc<Schema>, Box<dyn Iterator<Item = RecordBatch> + Send + 'static>) { (self.schema, self.batches) }

    fn new(lib: Arc<VeloxDynLib>, handle: VeloxStreamHandle) -> Self {
        let (schema, batches) = drain_native_stream(lib.clone(), handle);
        let native = VeloxStreamNative { lib, handle, ended: true };
        Self { schema, batches: Box::new(batches.into_iter()), _native: Some(native) }
    }
}

struct VeloxStreamNative {
    lib: Arc<VeloxDynLib>,
    handle: VeloxStreamHandle,
    #[allow(dead_code)]
    ended: bool,
}

impl Drop for VeloxStreamNative {
    fn drop(&mut self) {
        unsafe { (self.lib.stream_free)(self.handle) };
    }
}

fn drain_native_stream(lib: Arc<VeloxDynLib>, handle: VeloxStreamHandle) -> (Arc<Schema>, Vec<RecordBatch>) {
    let mut batches: Vec<RecordBatch> = Vec::new();
    let mut out_schema = Arc::new(Schema::empty());
    let mut have_schema = false;
    loop {
        // Prepare output FFI holders
        let mut c_schema = std::mem::MaybeUninit::<ArrowSchemaFFI>::uninit();
        let mut c_arr = std::mem::MaybeUninit::<ArrowArrayFFI>::uninit();
        let mut out_num_cols: c_int = 0;
        let mut out_num_rows: c_longlong = 0;
        let mut out_end: c_uchar = 0;
        let mut err = vec![0i8; 256];
        let got = unsafe {
            (lib.next)(
                handle,
                c_schema.as_mut_ptr(),
                c_arr.as_mut_ptr(),
                &mut out_num_cols,
                &mut out_num_rows,
                &mut out_end,
                err.as_mut_ptr() as *mut c_char,
                err.len(),
            )
        };
        if out_end != 0 || got == 0 {
            break;
        }
        let c_schema = unsafe { c_schema.assume_init() };
        let c_arr = unsafe { c_arr.assume_init() };
        // Import array and derive schema from it on first batch
        let array_data = match unsafe { from_ffi(c_arr, &c_schema) } {
            Ok(d) => d,
            Err(_) => break,
        };
        let array = make_array(array_data);
        if !have_schema {
            if let Some(sa) = array.as_any().downcast_ref::<arrow::array::StructArray>() {
                out_schema = Arc::new(arrow_schema::Schema::new(sa.fields().clone()));
                have_schema = true;
                let cols: Vec<ArrayRef> = (0..sa.num_columns()).map(|i| sa.column(i).clone()).collect();
                if let Ok(rb) = RecordBatch::try_new(out_schema.clone(), cols) { batches.push(rb); }
            } else {
                // Single column batch; synthesize schema
                let f = arrow_schema::Field::new("col0", array.data_type().clone(), true);
                out_schema = Arc::new(arrow_schema::Schema::new(vec![Arc::new(f)]));
                have_schema = true;
                if let Ok(rb) = RecordBatch::try_new(out_schema.clone(), vec![array.clone()]) { batches.push(rb); }
            }
        } else {
            if let Some(sa) = array.as_any().downcast_ref::<arrow::array::StructArray>() {
                let cols: Vec<ArrayRef> = (0..sa.num_columns()).map(|i| sa.column(i).clone()).collect();
                if let Ok(rb) = RecordBatch::try_new(out_schema.clone(), cols) { batches.push(rb); }
            } else {
                if let Ok(rb) = RecordBatch::try_new(out_schema.clone(), vec![array.clone()]) { batches.push(rb); }
            }
        }
    }
    (out_schema, batches)
}

struct VeloxDynLib {
    #[allow(dead_code)]
    lib: libloading::Library,
    create: VeloxCreateSession,
    #[allow(dead_code)]
    free: VeloxFreeSession,
    register: VeloxRegisterTableArrow,
    exec: VeloxExecuteSubstrait,
    next: VeloxStreamNext,
    stream_free: VeloxStreamFree,
}

impl VeloxDynLib {
    fn load() -> Option<Self> {
        // Try explicit env first
        let candidates = [
            std::env::var("VELOX_FFI_LIB").ok(),
            Some("libvelox_shim.so".to_string()),
        ];
        for cand in candidates.into_iter().flatten() {
            if let Ok(lib) = unsafe { libloading::Library::new(&cand) } {
                unsafe {
                    let create: VeloxCreateSession = *lib.get(b"velox_create_session\0").ok()?;
                    let free: VeloxFreeSession = *lib.get(b"velox_free_session\0").ok()?;
                    let register: VeloxRegisterTableArrow = *lib.get(b"velox_register_table_arrow\0").ok()?;
                    let exec: VeloxExecuteSubstrait = *lib.get(b"velox_execute_substrait\0").ok()?;
                    let next: VeloxStreamNext = *lib.get(b"velox_stream_next\0").ok()?;
                    let stream_free: VeloxStreamFree = *lib.get(b"velox_stream_free\0").ok()?;
                    return Some(Self { lib, create, free, register, exec, next, stream_free });
                }
            }
        }
        None
    }
}

// Helper to interpret C error buffer (unused in placeholder implementation)
#[allow(dead_code)]
fn cstr_err_to_string(buf: &[i8]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let bytes: Vec<u8> = buf[..n].iter().map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).to_string()
}


