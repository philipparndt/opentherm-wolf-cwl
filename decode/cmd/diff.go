package cmd

import (
	"fmt"
	"path/filepath"
	"strings"

	"decode/decoder"
	"decode/reader"

	"github.com/spf13/cobra"
)

var diffCmd = &cobra.Command{
	Use:   "diff <file-a> <file-b>",
	Short: "Compare two captures and highlight differences",
	Long:  "Decode two capture files and show a side-by-side comparison, highlighting values that changed between captures.",
	Args:  cobra.ExactArgs(2),
	RunE: func(cmd *cobra.Command, args []string) error {
		samplesA, err := loadFile(args[0])
		if err != nil {
			return fmt.Errorf("file A: %w", err)
		}
		samplesB, err := loadFile(args[1])
		if err != nil {
			return fmt.Errorf("file B: %w", err)
		}

		resultA := decoder.ExtractExchanges(samplesA)
		resultB := decoder.ExtractExchanges(samplesB)

		printDiff(args[0], args[1], resultA, resultB)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(diffCmd)
}

func loadFile(path string) ([]decoder.Sample, error) {
	ext := strings.ToLower(filepath.Ext(path))
	switch ext {
	case ".csv":
		return reader.LoadCSV(path)
	case ".sal":
		return reader.LoadSAL(path)
	default:
		return nil, fmt.Errorf("unsupported file format %q (supported: .csv, .sal)", ext)
	}
}

type exchangeEntry struct {
	name  string
	valA  string
	valB  string
	rawA  uint16
	rawB  uint16
	seenA bool
	seenB bool
}

func printDiff(nameA, nameB string, a, b decoder.CaptureResult) {
	// Deduplicate exchanges by key, keeping last value seen
	mapA := make(map[string]decoder.Exchange)
	orderA := []string{}
	for _, ex := range a.Exchanges {
		key := decoder.ExchangeKey(ex)
		if _, exists := mapA[key]; !exists {
			orderA = append(orderA, key)
		}
		mapA[key] = ex
	}

	mapB := make(map[string]decoder.Exchange)
	orderB := []string{}
	for _, ex := range b.Exchanges {
		key := decoder.ExchangeKey(ex)
		if _, exists := mapB[key]; !exists {
			orderB = append(orderB, key)
		}
		mapB[key] = ex
	}

	// Merge keys preserving order from A, then any new keys from B
	seen := make(map[string]bool)
	var allKeys []string
	for _, k := range orderA {
		if !seen[k] {
			allKeys = append(allKeys, k)
			seen[k] = true
		}
	}
	for _, k := range orderB {
		if !seen[k] {
			allKeys = append(allKeys, k)
			seen[k] = true
		}
	}

	// Build entries
	var entries []exchangeEntry
	for _, key := range allKeys {
		entry := exchangeEntry{}

		if exA, ok := mapA[key]; ok {
			entry.seenA = true
			entry.rawA = exA.Value
			entry.valA = decoder.FormatExchange(exA)
			entry.name = decoder.DisplayName(exA.DataID)
		}
		if exB, ok := mapB[key]; ok {
			entry.seenB = true
			entry.rawB = exB.Value
			entry.valB = decoder.FormatExchange(exB)
			if entry.name == "" {
				entry.name = decoder.DisplayName(exB.DataID)
			}
		}

		entries = append(entries, entry)
	}

	// Find column widths
	maxName := 0
	maxValA := len(filepath.Base(nameA))
	maxValB := len(filepath.Base(nameB))
	for _, e := range entries {
		if len(e.name) > maxName {
			maxName = len(e.name)
		}
		if len(e.valA) > maxValA {
			maxValA = len(e.valA)
		}
		if len(e.valB) > maxValB {
			maxValB = len(e.valB)
		}
	}

	// Header
	fmt.Printf("%-*s  %-*s  %-*s  %s\n",
		maxName+1, "",
		maxValA, filepath.Base(nameA),
		maxValB, filepath.Base(nameB),
		"")
	fmt.Printf("%-*s  %-*s  %-*s  %s\n",
		maxName+1, "",
		maxValA, strings.Repeat("-", len(filepath.Base(nameA))),
		maxValB, strings.Repeat("-", len(filepath.Base(nameB))),
		"")

	// Print entries
	changed := 0
	for _, e := range entries {
		label := e.name + ":"
		valA := e.valA
		valB := e.valB

		if !e.seenA {
			valA = "-"
		}
		if !e.seenB {
			valB = "-"
		}

		marker := ""
		if !e.seenA || !e.seenB || e.rawA != e.rawB {
			marker = " <--"
			changed++
		}

		fmt.Printf("%-*s  %-*s  %-*s%s\n",
			maxName+1, label,
			maxValA, valA,
			maxValB, valB,
			marker)
	}

	fmt.Printf("\n%d of %d values changed\n", changed, len(entries))
}
