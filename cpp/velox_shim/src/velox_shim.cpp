#include "velox_shim.h"
#include <string.h>
#include <string>
#include <memory>
#include <unordered_map>
#include <mutex>

#ifndef VELOX_SHIM_STUB
#include <arrow/c/abi.h>
#include "velox/common/memory/Memory.h"
#include "velox/type/Type.h"
#include "velox/type/TypeFactories.h"
#include "velox/vector/BaseVector.h"
#include "velox/vector/ComplexVector.h"
#include "velox/vector/arrow/Bridge.h"
// TODO: include Substrait converter headers when wiring execution
// #include "velox/substrait/VeloxPlanConverter.h"
using namespace facebook::velox;
using facebook::velox::memory::MemoryPool;

struct Session {
    std::shared_ptr<memory::MemoryPool> pool;
    std::mutex mu;
    std::unordered_map<std::string, std::shared_ptr<RowVector>> tables;
};

struct Stream {
    std::shared_ptr<Session> session;
    std::vector<RowVectorPtr> results;
    size_t idx{0};
    bool schema_set{false};
};
#endif

// Minimal Arrow C structs (opaque for now)
struct ArrowSchema { int dummy; };
struct ArrowArray { int dummy; };

extern "C" {

VeloxSession velox_create_session(char* err, size_t err_len) {
#ifdef VELOX_SHIM_STUB
    (void)err; (void)err_len;
    return reinterpret_cast<VeloxSession>(0x1);
#else
    (void)err; (void)err_len;
    try {
        auto& mm = memory::MemoryManager::getInstance();
        auto pool = mm.addRootPool("embucket_velox_shim");
        auto* s = new Session{pool};
        return reinterpret_cast<VeloxSession>(s);
    } catch (...) {
        return nullptr;
    }
#endif
}

void velox_free_session(VeloxSession session) {
    #ifdef VELOX_SHIM_STUB
    (void)session;
    #else
    auto* s = reinterpret_cast<Session*>(session);
    delete s;
    #endif
}

bool velox_register_table_arrow(
    VeloxSession session,
    const char* name,
    const ArrowSchema* schema,
    const ArrowArray* const* columns,
    int32_t num_cols,
    int64_t num_rows,
    char* err,
    size_t err_len) {
    (void)err; (void)err_len;
#ifdef VELOX_SHIM_STUB
    (void)session; (void)name; (void)schema; (void)columns; (void)num_cols; (void)num_rows;
    return true;
#else
    if (session == nullptr || schema == nullptr || name == nullptr || columns == nullptr) {
        return false;
    }
    auto* s = reinterpret_cast<Session*>(session);
    try {
        std::vector<VectorPtr> children;
        children.reserve(static_cast<size_t>(num_cols));
        std::vector<std::string> fieldNames;
        fieldNames.reserve(static_cast<size_t>(num_cols));
        for (int32_t i = 0; i < num_cols; ++i) {
            const ArrowSchema* childSchema = schema->children[i];
            const ArrowArray* childArray = columns[i];
            if (childSchema == nullptr || childArray == nullptr) {
                return false;
            }
            // Import each column as a Velox Vector
            VectorPtr col = importFromArrow(*childSchema, *childArray, s->pool.get());
            children.push_back(col);
            fieldNames.emplace_back(childSchema->name ? childSchema->name : "");
        }
        // Build RowType from child types
        std::vector<TypePtr> types;
        types.reserve(children.size());
        for (auto& c : children) {
            types.push_back(c->type());
        }
        auto rowType = ROW(std::move(fieldNames), std::move(types));
        auto row = std::make_shared<RowVector>(s->pool.get(), rowType, BufferPtr(nullptr), (vector_size_t)num_rows, std::move(children));
        std::lock_guard<std::mutex> lg(s->mu);
        s->tables[std::string(name)] = std::move(row);
        return true;
    } catch (...) {
        return false;
    }
#endif
}

VeloxStream velox_execute_substrait(
    VeloxSession session,
    const uint8_t* plan,
    size_t len,
    char* err,
    size_t err_len) {
    (void)plan; (void)len; (void)err; (void)err_len;
#ifdef VELOX_SHIM_STUB
    (void)session;
    // Stub: return an opaque non-null stream pointer
    return reinterpret_cast<VeloxStream>(0x2);
#else
    if (session == nullptr) return nullptr;
    auto* s = reinterpret_cast<Session*>(session);
    try {
        // Placeholder: execute plan and collect results into RowVectors
        // TODO: Use Velox Substrait converter to build and run plan, populate results
        auto stream = new Stream{std::shared_ptr<Session>(s, [](Session*){}), {}, 0, false};
        return reinterpret_cast<VeloxStream>(stream);
    } catch (...) {
        return nullptr;
    }
#endif
}

bool velox_stream_next(
    VeloxStream stream,
    ArrowSchema* out_schema,
    ArrowArray* out_batch,
    int32_t* out_num_cols,
    int64_t* out_num_rows,
    uint8_t* out_end,
    char* err,
    size_t err_len) {
    (void)err; (void)err_len;
#ifdef VELOX_SHIM_STUB
    (void)stream; (void)out_schema; (void)out_batch; (void)out_num_cols; (void)out_num_rows;
    if (out_end) *out_end = 1;
    return false;
#else
    if (stream == nullptr) return false;
    auto* st = reinterpret_cast<Stream*>(stream);
    if (st->idx >= st->results.size()) {
        if (out_end) *out_end = 1;
        return false;
    }
    auto batch = st->results[st->idx++];
    if (!st->schema_set) {
        // export schema from RowVector type
        try {
            auto ffi_schema = velox::exportToArrow(batch->type());
            // Copy into out_schema
            *out_schema = *ffi_schema.release();
            st->schema_set = true;
        } catch (...) {
            return false;
        }
    }
    try {
        // Export RowVector to Arrow C
        velox::exportToArrow(batch, *out_batch);
        if (out_num_cols) *out_num_cols = static_cast<int32_t>(batch->childrenSize());
        if (out_num_rows) *out_num_rows = static_cast<int64_t>(batch->size());
        if (out_end) *out_end = 0;
        return true;
    } catch (...) {
        if (out_end) *out_end = 1;
        return false;
    }
#endif
}

void velox_stream_free(VeloxStream stream) {
    #ifdef VELOX_SHIM_STUB
    (void)stream;
    #else
    auto* st = reinterpret_cast<Stream*>(stream);
    delete st;
    #endif
}

} // extern "C"
