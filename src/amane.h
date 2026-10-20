#ifndef AMANE_H
#define AMANE_H

#if defined(AMANE_ALL)
    #define DYNAMIC_ARRAYS
    #define STRING_VIEW
    #define STRING_BUILDER
    #define UTF8
#endif

#if defined(DYNAMIC_ARRAYS) && !defined(DYNAMIC_ARRAYS_INCLUDED)
#define DYNAMIC_ARRAYS_INCLUDED

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>
#include <assert.h>
#include <stdalign.h>

typedef struct {
    size_t count;
    size_t capacity;
    alignas(max_align_t) char data[];
} _Arr_Header;

#define _arr_header(a) ((_Arr_Header*)((char*)(a) - offsetof(_Arr_Header, data)))

#define arr_len(a)  ((a) ? _arr_header(a)->count : 0)
#define arr_cap(a)  ((a) ? _arr_header(a)->capacity : 0)
#define arr_last(a) ((a)[arr_len(a) - 1])

static inline void* _arr_grow(void *arr, size_t needed, size_t item_size) 
{
    size_t cap = arr_cap(arr);
    if (cap >= needed) 
        return arr;

    size_t new_cap = cap ? cap * 2 : 16;
    if (new_cap < needed) 
        new_cap = needed;

    size_t total_size = offsetof(_Arr_Header, data) + (new_cap * item_size);
    _Arr_Header *new_header = (_Arr_Header*)realloc(arr ? _arr_header(arr) : NULL, total_size);

    if (!new_header) {
        perror("Out of memory");
        exit(EXIT_FAILURE);
    }

    if (!arr) 
        new_header->count = 0;

    new_header->capacity = new_cap;
    return new_header->data;
}

#define arr_free(a) \
    do {\
        if (a) {\
            free(_arr_header(a));\
            (a) = NULL;\
        }\
    } while(0)

#define arr_clear(a) \
    do {\
        if (a)\
            _arr_header(a)->count = 0;\
    } while(0)

#define arr_reserve(a, n) \
    ((a) = _arr_grow((a), (n), sizeof(*(a))))

#define arr_push(a, val) \
    do { \
        (a) = _arr_grow((a), arr_len(a) + 1, sizeof(*(a))); \
        (a)[_arr_header(a)->count++] = (val); \
    } while(0)

// Pops and evaluates to the popped element
#define arr_pop(a) \
    ((a)[--_arr_header(a)->count])

#define arr_del_unordered(a, j) \
    do { \
        size_t _j = (j); \
        (a)[_j] = (a)[--_arr_header(a)->count]; \
    } while(0)

#define arr_remove_at(a, i) \
    do { \
        size_t _i = (i); \
        memmove(&(a)[_i], &(a)[_i + 1], sizeof(*(a)) * (arr_len(a) - _i - 1)); \
        _arr_header(a)->count--; \
    } while(0)

#define arr_insert_at(a, i, val) \
    do { \
        size_t _at = (i); \
        arr_push(a, val); \
        memmove(&(a)[_at + 1], &(a)[_at], sizeof(*(a)) * (arr_len(a) - _at - 1)); \
        (a)[_at] = (val); \
    } while(0)

#define arr_sort(arr, cmp_func) \
    do { if (arr) qsort((arr), arr_len(arr), sizeof(*(arr)), (cmp_func)); } while(0)

#endif // DYNAMIC_ARRAYS


#if (defined(STRING_VIEW) || defined(STRING_IMPLEMENTATION)) && !defined(STRING_VIEW_INCLUDED)
#define STRING_VIEW_INCLUDED

#include <stdlib.h>
#include <string.h>
#include <ctype.h>

typedef struct {
    const char *data;
    size_t count;
} String_View;

#define SV(cstr_lit) \
    ((String_View){ .data = (cstr_lit), .count = sizeof(cstr_lit) - 1 })

#define SV_FMT "%.*s"
#define SV_ARG(sv) (int)(sv).count, (sv).data

static inline String_View sv_from_parts(const char *data, size_t count) 
{
    return (String_View) { 
        .data = data, 
        .count = count 
    };
}

static inline String_View sv_from_cstr(const char *cstr) 
{
    if (!cstr) 
        return (String_View) {0};

    return (String_View){ 
        .data = cstr, 
        .count = strlen(cstr) 
    };
}

static inline int sv_eq(String_View a, String_View b) 
{
    if (a.count != b.count) 
        return 0;

    return memcmp(a.data, b.data, a.count) == 0;
}

static inline int sv_starts_with(String_View sv, String_View prefix) 
{
    if (sv.count < prefix.count) 
        return 0;

    return memcmp(sv.data, prefix.data, prefix.count) == 0;
}

static inline int sv_ends_with(String_View sv, String_View suffix) 
{
    if (sv.count < suffix.count) 
        return 0;

    return memcmp(sv.data + (sv.count - suffix.count), suffix.data, suffix.count) == 0;
}

static inline void sv_chop_left(String_View *sv, size_t n) 
{
    if (n > sv->count) 
        n = sv->count;

    sv->data += n;
    sv->count -= n;
}

static inline void sv_chop_right(String_View *sv, size_t n) 
{
    if (n > sv->count) 
        n = sv->count;

    sv->count -= n;
}

static inline String_View sv_chop_by_delim(String_View *sv, char delim) 
{
    size_t j = 0;
    while (j < sv->count && sv->data[j] != delim)
        j++;

    String_View result = sv_from_parts(sv->data, j);

    if (j < sv->count) 
        sv_chop_left(sv, j + 1);
    else
        sv_chop_left(sv, j);

    return result;
}

static inline void sv_trim_left(String_View *sv) 
{
    while (sv->count > 0 && isspace((unsigned char)sv->data[0]))
        sv_chop_left(sv, 1);
}

static inline void sv_trim_right(String_View *sv) 
{
    while (sv->count > 0 && isspace((unsigned char)sv->data[sv->count - 1]))
        sv->count--;
}

static inline void sv_trim(String_View *sv) 
{
    sv_trim_left(sv);
    sv_trim_right(sv);
}

static inline char *sv_to_cstr(String_View sv) 
{
    char *res = (char *)malloc(sv.count + 1);

    if (!res) 
        return NULL;

    memcpy(res, sv.data, sv.count);
    res[sv.count] = '\0';

    return res;
}

#endif // STRING_VIEW


#if defined(STRING_BUILDER) && !defined(STRING_BUILDER_INCLUDED)
#define STRING_BUILDER_INCLUDED

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char *data;
    size_t count;
    size_t capacity;
} String_Builder;

static inline void sb_reserve(String_Builder *sb, size_t needed)
{
    if (sb->data && needed + 1 <= sb->capacity)
        return;

    size_t new_cap = sb->capacity ? sb->capacity : 32;
    while (new_cap < needed + 1)
        new_cap *= 2;

    char *data = (char *)realloc(sb->data, new_cap);
    if (!data) {
        perror("Out of memory");
        exit(EXIT_FAILURE);
    }

    if (!sb->data)
        data[0] = '\0';

    sb->data = data;
    sb->capacity = new_cap;
}

static inline const char *sb_cstr(const String_Builder *sb)
{
    return sb->data ? sb->data : "";
}

static inline void sb_clear(String_Builder *sb)
{
    sb_reserve(sb, 0);
    sb->data[0] = '\0';
    sb->count = 0;
}

static inline void sb_set(String_Builder *sb, const char *cstr)
{
    size_t n = strlen(cstr);

    sb_reserve(sb, n);
    memcpy(sb->data, cstr, n + 1);
    sb->count = n;
}

static inline void sb_insert(String_Builder *sb, size_t at, const char *s, size_t n)
{
    if (at > sb->count)
        at = sb->count;

    sb_reserve(sb, sb->count + n);
    memmove(sb->data + at + n, sb->data + at, sb->count - at + 1);
    memcpy(sb->data + at, s, n);
    sb->count += n;
}

static inline void sb_append(String_Builder *sb, const char *s, size_t n)
{
    sb_insert(sb, sb->count, s, n);
}

static inline void sb_putc(String_Builder *sb, char c)
{
    sb_insert(sb, sb->count, &c, 1);
}

static inline void sb_remove(String_Builder *sb, size_t at, size_t n)
{
    if (!sb->data || at >= sb->count)
        return;

    if (n > sb->count - at)
        n = sb->count - at;

    memmove(sb->data + at, sb->data + at + n, sb->count - at - n + 1);
    sb->count -= n;
}

static inline void sb_free(String_Builder *sb)
{
    free(sb->data);
    sb->data = NULL;
    sb->count = 0;
    sb->capacity = 0;
}

#endif // STRING_BUILDER


#if defined(UTF8) && !defined(UTF8_INCLUDED)
#define UTF8_INCLUDED

#include <stddef.h>
#include <stdint.h>
#include <wchar.h>

static inline int u8_dec(const char *s, size_t remaining, uint32_t *cp)
{
    unsigned char c = (unsigned char)s[0];

    if (c < 0x80) {
        *cp = c;
        return 1;
    }

    int len = c >= 0xF0 ? 4 : c >= 0xE0 ? 3 : c >= 0xC2 ? 2 : 0;
    if (!len || (size_t)len > remaining || c > 0xF4) {
        *cp = 0xFFFD;
        return 1;
    }

    uint32_t value = c & (0xFFu >> (len + 1));
    for (int j = 1; j < len; j++) {
        if (((unsigned char)s[j] & 0xC0) != 0x80) {
            *cp = 0xFFFD;
            return 1;
        }
        value = value << 6 | ((unsigned char)s[j] & 0x3F);
    }

    *cp = value;
    return len;
}

static inline int u8_enc(uint32_t cp, char *out)
{
    if (cp < 0x80) {
        out[0] = (char)cp;
        return 1;
    }

    if (cp < 0x800) {
        out[0] = (char)(0xC0 | cp >> 6);
        out[1] = (char)(0x80 | (cp & 0x3F));
        return 2;
    }

    if (cp < 0x10000) {
        out[0] = (char)(0xE0 | cp >> 12);
        out[1] = (char)(0x80 | (cp >> 6 & 0x3F));
        out[2] = (char)(0x80 | (cp & 0x3F));
        return 3;
    }

    out[0] = (char)(0xF0 | cp >> 18);
    out[1] = (char)(0x80 | (cp >> 12 & 0x3F));
    out[2] = (char)(0x80 | (cp >> 6 & 0x3F));
    out[3] = (char)(0x80 | (cp & 0x3F));
    return 4;
}

static inline size_t u8_next(const char *s, size_t count, size_t i)
{
    uint32_t cp;

    if (i >= count)
        return count;

    return i + (size_t)u8_dec(s + i, count - i, &cp);
}

static inline size_t u8_prev(const char *s, size_t i)
{
    if (i == 0)
        return 0;

    i--;
    while (i > 0 && ((unsigned char)s[i] & 0xC0) == 0x80)
        i--;

    return i;
}

static inline size_t u8_chars(const char *s, size_t count)
{
    size_t n = 0;

    for (size_t i = 0; i < count; i = u8_next(s, count, i))
        n++;

    return n;
}

#ifdef _WIN32
static inline int u8_in_ranges(uint32_t cp, const uint32_t (*ranges)[2], size_t count)
{
    for (size_t j = 0; j < count; j++)
        if (cp >= ranges[j][0] && cp <= ranges[j][1])
            return 1;

    return 0;
}

static inline int u8_cp_width(uint32_t cp)
{
    static const uint32_t zero[][2] = {
        {0x0300, 0x036F}, {0x200B, 0x200F}, {0x20D0, 0x20FF}, {0xFE00, 0xFE0F},
    };
    static const uint32_t wide[][2] = {
        {0x1100, 0x115F},   {0x2E80, 0x303E},   {0x3041, 0x33FF},   {0x3400, 0x4DBF},
        {0x4E00, 0x9FFF},   {0xA000, 0xA4CF},   {0xAC00, 0xD7A3},   {0xF900, 0xFAFF},
        {0xFE30, 0xFE6F},   {0xFF00, 0xFF60},   {0xFFE0, 0xFFE6},   {0x1F300, 0x1F64F},
        {0x1F900, 0x1F9FF}, {0x20000, 0x3FFFD},
    };

    if (cp < 0x300)
        return 1;

    if (u8_in_ranges(cp, zero, sizeof zero / sizeof *zero))
        return 0;

    return u8_in_ranges(cp, wide, sizeof wide / sizeof *wide) ? 2 : 1;
}
#else
static inline int u8_cp_width(uint32_t cp)
{
    if (cp <= 0x7F)
        return 1;

    int w = wcwidth((wchar_t)cp);
    return w < 0 ? 1 : w;
}
#endif

static inline int u8_width(const char *s, size_t count)
{
    int w = 0;

    for (size_t i = 0; i < count;) {
        uint32_t cp;

        i += (size_t)u8_dec(s + i, count - i, &cp);
        w += u8_cp_width(cp);
    }

    return w;
}

#endif // UTF8

#endif // AMANE_H
