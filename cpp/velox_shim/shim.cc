#include "shim.h"

#include <string>
#include <cstring>
#include <new>

static void set_err(char* err, size_t err_len, const std::string& msg) {
  if (!err || err_len == 0) return;
  size_t n = msg.size();
  if (n >= err_len) n = err_len - 1;
  memcpy(err, msg.data(), n);
  err[n] = '\0';
}

// Define the opaque structs declared in shim.h
struct VeloxSession {
#ifdef VELOX_AVAILABLE
  // TODO: wire real Velox context objects (memory pool, exec ctx, function registry)
#endif
  int ok;
};

struct VeloxStream {
  int done;
};

extern "C" {

VeloxSession* velox_create_session(char* err, size_t err_len) {
  auto* s = new (std::nothrow) VeloxSession();
  if (!s) {
    set_err(err, err_len, "Out of memory");
    return nullptr;
  }
#ifdef VELOX_AVAILABLE
  // Here we would construct Velox core context and mark ok=1 on success
  s->ok = 1;
#else
  // Velox not available: still return a non-null handle so Rust can detect availability via symbols
  s->ok = 0;
#endif
  return s;
}

void velox_free_session(VeloxSession* s) {
  delete s;
}

bool velox_register_table_arrow(
    VeloxSession* /*s*/,
    const char* /*name*/,
    const struct ArrowSchema* /*schema*/,
    const struct ArrowArray* const* /*columns*/,
    int32_t /*num_cols*/,
    int64_t /*num_rows*/,
    char* err,
    size_t err_len) {
#ifdef VELOX_SHIM_STUB
  set_err(err, err_len, "register_table not available in stub build");
  return false;
#else
  return false;
#endif
}

VeloxStream* velox_execute_substrait(
    VeloxSession* /*s*/,
    const uint8_t* /*plan*/,
    size_t /*len*/,
    char* err,
    size_t err_len) {
#ifdef VELOX_SHIM_STUB
  set_err(err, err_len, "execute_substrait not available in stub build");
  return nullptr;
#else
  return nullptr;
#endif
}

bool velox_stream_next(
    VeloxStream* st,
    struct ArrowSchema* /*out_schema*/,
    struct ArrowArray* /*out_batch*/,
    int32_t* /*out_num_cols*/,
    int64_t* /*out_num_rows*/,
    bool* out_end,
    char* /*err*/,
    size_t /*err_len*/) {
  if (!st || !out_end) return false;
  *out_end = true;
  return true;
}

void velox_stream_free(VeloxStream* st) {
  delete st;
}

} // extern "C"


