#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_DEPTH 3

static char *slurp(const char *path)
{
    FILE *f = fopen(path, "rb");
    long size;
    char *text;

    if (!f)
        return NULL;
    fseek(f, 0, SEEK_END);
    size = ftell(f);
    rewind(f);
    text = malloc((size_t)size + 1);
    if (text && fread(text, 1, (size_t)size, f) != (size_t)size)
        size = 0;
    if (text)
        text[size] = '\0';
    fclose(f);
    return text;
}

static int is_vendored(const char *path)
{
    size_t n = strlen(path);

    return n >= 7 && !strcmp(path + n - 7, "amane.h");
}

static size_t skip_literal(const char *s, size_t i)
{
    char quote = s[i++];

    while (s[i] && s[i] != quote)
        i += s[i] == '\\' && s[i + 1] ? 2 : 1;
    return s[i] ? i + 1 : i;
}

static size_t skip_comment(const char *s, size_t i)
{
    if (s[i + 1] == '/') {
        while (s[i] && s[i] != '\n')
            i++;
        return i;
    }
    i += 2;
    while (s[i] && !(s[i] == '*' && s[i + 1] == '/'))
        i++;
    return s[i] ? i + 2 : i;
}

static int starts_comment(const char *s, size_t i)
{
    return s[i] == '/' && (s[i + 1] == '/' || s[i + 1] == '*');
}

static int check_comments(const char *path, const char *text)
{
    int found = 0, line = 1;

    for (size_t i = 0; text[i];) {
        if (text[i] == '"' || text[i] == '\'') {
            i = skip_literal(text, i);
            continue;
        }
        if (starts_comment(text, i)) {
            printf("%s:%d: comment (write self-documenting code instead)\n", path, line);
            found++;
            i = skip_comment(text, i);
            continue;
        }
        line += text[i++] == '\n';
    }
    return found;
}

static int starts_statement(const char *code)
{
    while (*code == ' ')
        code++;
    return *code && !strchr("}.\"&|?:{", *code);
}

static int check_nesting_line(const char *path, int number, const char *line, int depth)
{
    if (depth <= MAX_DEPTH || !starts_statement(line))
        return 0;
    printf("%s:%d: nested %d deep (max %d): %s\n", path, number, depth, MAX_DEPTH, line);
    return 1;
}

static int brace_delta(const char *line)
{
    int delta = 0;

    for (size_t i = 0; line[i]; i++) {
        if (line[i] == '"' || line[i] == '\'')
            i = skip_literal(line, i) - 1;
        else if (starts_comment(line, i))
            break;
        else
            delta += (line[i] == '{') - (line[i] == '}');
    }
    return delta;
}

static int check_nesting(const char *path, char *text)
{
    int found = 0, depth = 0, number = 0;

    for (char *line = text, *next; line && *line; line = next) {
        next = strchr(line, '\n');
        if (next)
            *next++ = '\0';
        number++;
        found += check_nesting_line(path, number, line, depth);
        depth += brace_delta(line);
    }
    return found;
}

static int is_word(char c)
{
    return c == '_' || (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

static int banned_word(const char *s, size_t i)
{
    static const char *const words[] = {"bool", "true", "false", "stdbool"};

    if (i && is_word(s[i - 1]))
        return 0;
    for (size_t w = 0; w < sizeof words / sizeof *words; w++) {
        size_t n = strlen(words[w]);

        if (!strncmp(s + i, words[w], n) && !is_word(s[i + n]))
            return 1;
    }
    return 0;
}

static int check_types(const char *path, const char *text)
{
    int found = 0, line = 1;

    for (size_t i = 0; text[i];) {
        if (text[i] == '"' || text[i] == '\'') {
            i = skip_literal(text, i);
            continue;
        }
        if (starts_comment(text, i)) {
            i = skip_comment(text, i);
            continue;
        }
        if (banned_word(text, i)) {
            printf("%s:%d: bool is a lie, use int\n", path, line);
            found++;
        }
        line += text[i] == '\n';
        i++;
    }
    return found;
}

static int run_check(const char *mode, const char *path, char *text)
{
    if (!strcmp(mode, "nesting"))
        return check_nesting(path, text);
    if (!strcmp(mode, "types"))
        return check_types(path, text);
    return check_comments(path, text);
}

int main(int argc, char **argv)
{
    int failed = 0;

    if (argc < 3 || (strcmp(argv[1], "comments") && strcmp(argv[1], "nesting") && strcmp(argv[1], "types"))) {
        fprintf(stderr, "usage: lint comments|nesting|types file...\n");
        return 2;
    }
    for (int i = 2; i < argc; i++) {
        char *text = is_vendored(argv[i]) ? NULL : slurp(argv[i]);

        if (!text)
            continue;
        failed += run_check(argv[1], argv[i], text);
        free(text);
    }
    return failed != 0;
}
