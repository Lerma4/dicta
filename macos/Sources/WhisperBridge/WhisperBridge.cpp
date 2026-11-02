#include "WhisperBridge.h"
#include "whisper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

struct DictaWhisper {
    whisper_context *context;
};

namespace {
void discard_log(ggml_log_level, const char *, void *) {}
}

extern "C" DictaWhisper *dicta_whisper_create(const char *model_path) {
    if (model_path == nullptr) return nullptr;
    whisper_log_set(discard_log, nullptr);
    ggml_log_set(discard_log, nullptr);
    whisper_context_params context_params = whisper_context_default_params();
    context_params.use_gpu = false;
    whisper_context *context = whisper_init_from_file_with_params(model_path, context_params);
    if (context == nullptr) return nullptr;
    auto *engine = new (std::nothrow) DictaWhisper{context};
    if (engine == nullptr) whisper_free(context);
    return engine;
}

extern "C" void dicta_whisper_destroy(DictaWhisper *engine) {
    if (engine == nullptr) return;
    whisper_free(engine->context);
    delete engine;
}

extern "C" char *dicta_whisper_transcribe(DictaWhisper *engine, const float *samples, size_t count, const char *language) {
    if (engine == nullptr || samples == nullptr || count == 0 || count > static_cast<size_t>(INT_MAX) || language == nullptr || (std::strcmp(language, "auto") != 0 && whisper_lang_id(language) < 0)) return nullptr;
    whisper_full_params params = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    params.language = language;
    params.translate = false;
    params.n_threads = 4;
    params.print_realtime = false;
    params.print_progress = false;
    params.print_timestamps = false;
    params.print_special = false;
    params.no_context = true;
    params.single_segment = false;
    if (whisper_full(engine->context, params, samples, static_cast<int>(count)) != 0) return nullptr;

    std::string result;
    const int segments = whisper_full_n_segments(engine->context);
    for (int index = 0; index < segments; ++index) {
        const char *segment = whisper_full_get_segment_text(engine->context, index);
        if (segment != nullptr) result += segment;
    }
    auto *copy = static_cast<char *>(std::malloc(result.size() + 1));
    if (copy == nullptr) return nullptr;
    std::memcpy(copy, result.c_str(), result.size() + 1);
    return copy;
}

extern "C" void dicta_whisper_free_text(char *text) { std::free(text); }
