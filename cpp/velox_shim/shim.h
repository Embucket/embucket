#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Arrow C Data Interface (forward declarations)
struct ArrowSchema;
struct ArrowArray;

// Opaque handles
typedef struct VeloxSession VeloxSession;
typedef struct VeloxStream VeloxStream;

#ifdef __cplusplus
extern "C" {
#endif

VeloxSession* velox_create_session(char* err, size_t err_len);
void velox_free_session(VeloxSession* s);

bool velox_register_table_arrow(
    VeloxSession* s,
    const char* name,
    const struct ArrowSchema* schema,
    const struct ArrowArray* const* columns,
    int32_t num_cols,
    int64_t num_rows,
    char* err,
    size_t err_len);

VeloxStream* velox_execute_substrait(
    VeloxSession* s,
    const uint8_t* plan,
    size_t len,
    char* err,
    size_t err_len);

bool velox_stream_next(
    VeloxStream* st,
    struct ArrowSchema* out_schema,
    struct ArrowArray* out_batch,
    int32_t* out_num_cols,
    int64_t* out_num_rows,
    bool* out_end,
    char* err,
    size_t err_len);

void velox_stream_free(VeloxStream* st);

#ifdef __cplusplus
} // extern "C"
#endif


