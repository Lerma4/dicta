#ifndef DICTA_WHISPER_BRIDGE_H
#define DICTA_WHISPER_BRIDGE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DictaWhisper DictaWhisper;

DictaWhisper *dicta_whisper_create(const char *model_path);
void dicta_whisper_destroy(DictaWhisper *engine);
char *dicta_whisper_transcribe(DictaWhisper *engine, const float *samples, size_t count, const char *language);
void dicta_whisper_free_text(char *text);

#ifdef __cplusplus
}
#endif
#endif
