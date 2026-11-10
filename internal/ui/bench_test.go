package ui

import "testing"

func BenchmarkCanvasRender(b *testing.B) {
	b.ReportAllocs()
	for b.Loop() {
		c := NewCanvas(200, 50)
		for y := 0; y < 50; y++ {
			c.Text(0, y, 200, "Some ordinary ASCII task title that fills the row nicely, with more", Style{Fg: Ansi(2)})
			c.Paint(Rect{0, y, 200, 1}, Style{Bg: Ansi(4)})
		}
		_ = c.ANSI()
	}
}
