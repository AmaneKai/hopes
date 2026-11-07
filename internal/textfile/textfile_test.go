package textfile

import "testing"

func encodeUTF16(bom []byte, big bool, units ...uint16) string {
	out := append([]byte{}, bom...)
	for _, u := range units {
		hi, lo := byte(u>>8), byte(u)
		if big {
			out = append(out, hi, lo)
		} else {
			out = append(out, lo, hi)
		}
	}
	return string(out)
}

var (
	littleBOM = []byte{0xFF, 0xFE}
	bigBOM    = []byte{0xFE, 0xFF}
)

func TestNormalizeHandlesEveryEncoding(t *testing.T) {
	cases := map[string]string{
		"\xEF\xBB\xBF[]":                      "[]",
		"[]":                                  "[]",
		encodeUTF16(bigBOM, true, 0x5B, 0x5D): "[]",
		encodeUTF16(littleBOM, false, 0x22, 0xD83D, 0xDE00): "\"\U0001F600",
		encodeUTF16(littleBOM, false, 0xD800, 0x78):         "\uFFFDx",
		"\xFF\xFEa\x00b": "a",
		"\xFF\xFE":       "",
		"\xEF\xBB\xBF":   "",
		"":               "",
	}
	for in, want := range cases {
		if got := Normalize([]byte(in)); got != want {
			t.Errorf("Normalize(%q) = %q, want %q", in, got, want)
		}
	}
}
