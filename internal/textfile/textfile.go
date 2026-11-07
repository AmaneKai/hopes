package textfile

import (
	"bytes"
	"strings"
	"unicode/utf16"
)

const replacement = "\uFFFD"

func Normalize(data []byte) string {
	switch {
	case bytes.HasPrefix(data, []byte{0xEF, 0xBB, 0xBF}):
		return strings.ToValidUTF8(string(data[3:]), replacement)
	case bytes.HasPrefix(data, []byte{0xFF, 0xFE}):
		return decodeUTF16(data[2:], littleEndian)
	case bytes.HasPrefix(data, []byte{0xFE, 0xFF}):
		return decodeUTF16(data[2:], bigEndian)
	}
	return strings.ToValidUTF8(string(data), replacement)
}

func decodeUTF16(data []byte, unit func([]byte) uint16) string {
	units := make([]uint16, 0, len(data)/2)
	for i := 0; i+1 < len(data); i += 2 {
		units = append(units, unit(data[i:i+2]))
	}
	return string(utf16.Decode(units))
}

func littleEndian(b []byte) uint16 { return uint16(b[0]) | uint16(b[1])<<8 }

func bigEndian(b []byte) uint16 { return uint16(b[1]) | uint16(b[0])<<8 }
