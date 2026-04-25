package cmd

import (
	"github.com/spf13/cobra"
)

var csvCmd = &cobra.Command{
	Use:   "csv <file>",
	Short: "Decode from Saleae CSV export",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		return decodeCSV(args[0])
	},
}

func init() {
	rootCmd.AddCommand(csvCmd)
}
