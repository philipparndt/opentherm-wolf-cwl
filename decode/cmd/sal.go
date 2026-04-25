package cmd

import (
	"github.com/spf13/cobra"
)

var salCmd = &cobra.Command{
	Use:   "sal <file>",
	Short: "Decode from Saleae .sal capture file",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		return decodeSAL(args[0])
	},
}

func init() {
	rootCmd.AddCommand(salCmd)
}
