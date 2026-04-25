package reader

import (
	"encoding/csv"
	"os"
	"strconv"

	"decode/decoder"
)

func LoadCSV(path string) ([]decoder.Sample, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	r := csv.NewReader(f)
	if _, err := r.Read(); err != nil {
		return nil, err
	}

	var samples []decoder.Sample
	// Read first two samples to determine sample rate for downsampling
	var firstTime, secondTime float64
	lineNum := 0
	for {
		record, err := r.Read()
		if err != nil {
			break
		}
		t, err := strconv.ParseFloat(record[0], 64)
		if err != nil {
			continue
		}
		v, err := strconv.ParseFloat(record[1], 64)
		if err != nil {
			continue
		}
		lineNum++
		if lineNum == 1 {
			firstTime = t
		} else if lineNum == 2 {
			secondTime = t
		}
		samples = append(samples, decoder.Sample{Time: t, Voltage: v})
	}

	// Downsample if sample rate is high enough
	if len(samples) > 2 {
		interval := secondTime - firstTime
		if interval > 0 {
			sampleRate := 1.0 / interval
			step := int(sampleRate / 150000)
			if step > 1 {
				downsampled := make([]decoder.Sample, 0, len(samples)/step+1)
				for i := 0; i < len(samples); i += step {
					downsampled = append(downsampled, samples[i])
				}
				samples = downsampled
			}
		}
	}

	return samples, nil
}
