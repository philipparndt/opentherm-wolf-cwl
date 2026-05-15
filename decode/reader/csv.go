package reader

import (
	"encoding/csv"
	"os"
	"strconv"

	"decode/decoder"
)

type rawRow struct {
	t, v float64
}

// LoadCSV reads either an analog Saleae CSV (Time, Voltage at fixed intervals)
// or a digital Saleae CSV (Time, 0/1 at transition timestamps only). Digital
// captures are expanded to regularly-spaced samples and rescaled so the
// existing analog Manchester decoder pipeline can consume them unchanged.
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

	var rows []rawRow
	allBinary := true
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
		if v != 0 && v != 1 {
			allBinary = false
		}
		rows = append(rows, rawRow{t, v})
	}

	if len(rows) == 0 {
		return nil, nil
	}

	if allBinary {
		return expandDigital(rows), nil
	}
	return downsampleAnalog(rows), nil
}

// expandDigital takes Saleae digital transition rows (time, 0|1) and produces
// regularly-spaced analog-shaped samples. The signal is inverted if the
// dominant state is HIGH (so idle reads as low voltage like an OT bus capture
// where the master idles by not sinking current). Scaled to 0/20 V so the
// existing decoder thresholds (6.1 V findPackets, 6.5/8.0 V Manchester edges)
// work without modification.
func expandDigital(rows []rawRow) []decoder.Sample {
	// Time-weighted dominant state — long idle stretches dwarf short bursts,
	// so the dominant state ≈ the idle state. We invert if that's HIGH.
	highTime, lowTime := 0.0, 0.0
	for i := 0; i < len(rows)-1; i++ {
		dt := rows[i+1].t - rows[i].t
		if rows[i].v == 1 {
			highTime += dt
		} else {
			lowTime += dt
		}
	}
	invert := highTime > lowTime

	// Sample at 100 kHz — gives 50 samples per Manchester half-bit (500 µs),
	// plenty for clean edge detection while keeping the synthesized buffer
	// reasonable for multi-second captures.
	const sampleHz = 100_000.0
	const dt = 1.0 / sampleHz
	const activeV = 20.0
	const idleV = 0.0

	start := rows[0].t
	end := rows[len(rows)-1].t
	if end <= start {
		return nil
	}

	samples := make([]decoder.Sample, 0, int((end-start)*sampleHz)+1)
	rowIdx := 0
	state := rows[0].v
	for t := start; t <= end; t += dt {
		for rowIdx+1 < len(rows) && rows[rowIdx+1].t <= t {
			rowIdx++
			state = rows[rowIdx].v
		}
		s := state
		if invert {
			s = 1 - s
		}
		voltage := idleV
		if s == 1 {
			voltage = activeV
		}
		samples = append(samples, decoder.Sample{Time: t, Voltage: voltage})
	}
	return samples
}

func downsampleAnalog(rows []rawRow) []decoder.Sample {
	samples := make([]decoder.Sample, 0, len(rows))
	for _, r := range rows {
		samples = append(samples, decoder.Sample{Time: r.t, Voltage: r.v})
	}
	if len(samples) > 2 {
		interval := samples[1].Time - samples[0].Time
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
	return samples
}
