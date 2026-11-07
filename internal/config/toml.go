package config

import (
	"errors"
	"strconv"
	"strings"
)

type table map[string]any

var errSyntax = errors.New("config syntax error")

func parseTOML(text string) (map[string]table, error) {
	doc := map[string]table{"": {}}
	current := doc[""]
	lines := strings.Split(strings.ReplaceAll(text, "\r\n", "\n"), "\n")
	for i := 0; i < len(lines); i++ {
		line := strings.TrimSpace(stripComment(lines[i]))
		for openArray(line) && i+1 < len(lines) {
			i++
			line += " " + strings.TrimSpace(stripComment(lines[i]))
		}
		next, err := parseLine(doc, current, line)
		if err != nil {
			return nil, err
		}
		current = next
	}
	return doc, nil
}

func openArray(line string) bool {
	_, value, ok := strings.Cut(line, "=")
	value = strings.TrimSpace(value)
	return ok && strings.HasPrefix(value, "[") && !strings.HasSuffix(value, "]")
}

func parseLine(doc map[string]table, current table, line string) (table, error) {
	if line == "" {
		return current, nil
	}
	if strings.HasPrefix(line, "[") {
		return parseHeader(doc, line)
	}
	key, raw, ok := strings.Cut(line, "=")
	key = strings.TrimSpace(key)
	if !ok || key == "" {
		return nil, errSyntax
	}
	if _, dup := current[key]; dup {
		return nil, errSyntax
	}
	value, err := parseValue(strings.TrimSpace(raw))
	current[key] = value
	return current, err
}

func parseHeader(doc map[string]table, line string) (table, error) {
	name, ok := strings.CutSuffix(strings.TrimPrefix(line, "["), "]")
	name = strings.TrimSpace(name)
	if !ok || name == "" || doc[name] != nil {
		return nil, errSyntax
	}
	doc[name] = table{}
	return doc[name], nil
}

func parseValue(raw string) (any, error) {
	switch {
	case raw == "true" || raw == "false":
		return raw == "true", nil
	case strings.HasPrefix(raw, "["):
		return parseArray(raw)
	case strings.HasPrefix(raw, `"`), strings.HasPrefix(raw, "'"):
		return parseString(raw)
	}
	n, err := strconv.ParseInt(strings.ReplaceAll(raw, "_", ""), 10, 64)
	if err != nil {
		return nil, errSyntax
	}
	return n, nil
}

func parseString(raw string) (string, error) {
	if strings.HasPrefix(raw, "'") {
		s, ok := strings.CutSuffix(raw[1:], "'")
		if !ok || strings.Contains(s, "'") {
			return "", errSyntax
		}
		return s, nil
	}
	s, err := strconv.Unquote(raw)
	if err != nil {
		return "", errSyntax
	}
	return s, nil
}

func parseArray(raw string) ([]any, error) {
	body, ok := strings.CutSuffix(strings.TrimPrefix(raw, "["), "]")
	if !ok {
		return nil, errSyntax
	}
	values := []any{}
	for _, part := range splitArray(body) {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}
		v, err := parseValue(part)
		if err != nil {
			return nil, err
		}
		values = append(values, v)
	}
	return values, nil
}

func splitArray(body string) []string {
	var parts []string
	var quote rune
	start := 0
	for i, r := range body {
		quote = trackQuote(quote, r, body, i)
		if r == ',' && quote == 0 {
			parts = append(parts, body[start:i])
			start = i + 1
		}
	}
	return append(parts, body[start:])
}

func trackQuote(quote, r rune, body string, i int) rune {
	escaped := i > 0 && body[i-1] == '\\' && quote == '"'
	switch {
	case quote == 0 && (r == '"' || r == '\''):
		return r
	case r == quote && !escaped:
		return 0
	}
	return quote
}

func stripComment(line string) string {
	var quote rune
	for i, r := range line {
		quote = trackQuote(quote, r, line, i)
		if r == '#' && quote == 0 {
			return line[:i]
		}
	}
	return line
}
