#pragma once
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Arrow C Data Interface (minimal forward declarations)
typedef struct ArrowSchema ArrowSchema;
typedef struct ArrowArray ArrowArray;

typedef void* VeloxSession;
typedef void* VeloxStream;

// Creates a new Velox session. Writes an error message into `err` (UTF-8) if non-null.
VeloxSession velox_create_session(char* err, size_t err_len);

// Frees a session.
void velox_free_session(VeloxSession session);

// Registers a table from Arrow schema + columns (single batch). Returns true on success.
bool velox_register_table_arrow(
    VeloxSession session,
    const char* name,
    const ArrowSchema* schema,
    const ArrowArray* const* columns,
    int32_t num_cols,
    int64_t num_rows,
    char* err,
    size_t err_len);

// Executes a Substrait plan buffer and returns a stream handle (or NULL on error).
VeloxStream velox_execute_substrait(
    VeloxSession session,
    const uint8_t* plan,
    size_t len,
    char* err,
    size_t err_len);

// Retrieves the next batch from the stream. Returns true if a batch is produced, false on end or error.
bool velox_stream_next(
    VeloxStream stream,
    ArrowSchema* out_schema,
    ArrowArray* out_batch,
    int32_t* out_num_cols,
    int64_t* out_num_rows,
    uint8_t* out_end,
    char* err,
    size_t err_len);

// Frees the stream
typedef void (*velox_stream_free_fn)(VeloxStream stream);
void velox_stream_free(VeloxStream stream);

#ifdef __cplusplus
}
#endif
