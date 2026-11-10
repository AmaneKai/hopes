package storage

import (
	"os"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/textfile"
)

func ReadItemsFile(path string) ([]domain.Item, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	return decodeItems(textfile.Normalize(data))
}
