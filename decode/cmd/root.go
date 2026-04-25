package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"decode/decoder"
	"decode/reader"

	"github.com/spf13/cobra"
)

var verbose bool

var rootCmd = &cobra.Command{
	Use:   "decode [file]",
	Short: "Decode OpenTherm protocol from Saleae recordings",
	Long:  "Decode OpenTherm Manchester-encoded frames from Saleae Logic 2 captures (.sal) or CSV exports (.csv).",
	Args:  cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		if len(args) == 0 {
			return cmd.Help()
		}
		path := args[0]
		ext := strings.ToLower(filepath.Ext(path))
		switch ext {
		case ".csv":
			return decodeCSV(path)
		case ".sal":
			return decodeSAL(path)
		default:
			return fmt.Errorf("unsupported file format %q (supported: .csv, .sal)", ext)
		}
	},
}

func init() {
	rootCmd.PersistentFlags().BoolVarP(&verbose, "verbose", "v", false, "Show verbose output with raw hex, parity, and all numeric representations")
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}

func decodeCSV(path string) error {
	samples, err := reader.LoadCSV(path)
	if err != nil {
		return fmt.Errorf("error loading CSV: %w", err)
	}
	decodeSamples(samples)
	return nil
}

func decodeSAL(path string) error {
	samples, err := reader.LoadSAL(path)
	if err != nil {
		return fmt.Errorf("error loading SAL: %w", err)
	}
	decodeSamples(samples)
	return nil
}

func decodeSamples(samples []decoder.Sample) {
	if verbose {
		decoder.Decode(samples)
	} else {
		decoder.DecodeReadable(samples)
	}
}
