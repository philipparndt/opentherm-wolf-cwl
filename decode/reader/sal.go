package reader

import (
	"archive/zip"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"

	"decode/decoder"
)

type salMeta struct {
	Data struct {
		LegacySettings struct {
			SampleRate struct {
				Analog float64 `json:"analog"`
			} `json:"sampleRate"`
		} `json:"legacySettings"`
		LegacyDeviceCalibration struct {
			FullScaleVoltageRanges []struct {
				MinimumVoltage float64 `json:"minimumVoltage"`
				MaximumVoltage float64 `json:"maximumVoltage"`
			} `json:"fullScaleVoltageRanges"`
		} `json:"legacyDeviceCalibration"`
	} `json:"data"`
}

func LoadSAL(path string) ([]decoder.Sample, error) {
	r, err := zip.OpenReader(path)
	if err != nil {
		return nil, fmt.Errorf("cannot open .sal file: %w", err)
	}
	defer r.Close()

	metaBytes, err := readZipFile(r, "meta.json")
	if err != nil {
		return nil, fmt.Errorf("cannot read meta.json from .sal: %w", err)
	}

	var meta salMeta
	if err := json.Unmarshal(metaBytes, &meta); err != nil {
		return nil, fmt.Errorf("cannot parse meta.json: %w", err)
	}

	sampleRate := meta.Data.LegacySettings.SampleRate.Analog
	if sampleRate <= 0 {
		return nil, fmt.Errorf("invalid or missing sample rate in meta.json (got %.0f)", sampleRate)
	}

	// Get voltage calibration for channel 0
	calRanges := meta.Data.LegacyDeviceCalibration.FullScaleVoltageRanges
	if len(calRanges) == 0 {
		return nil, fmt.Errorf("no calibration data found in meta.json")
	}
	vmin := calRanges[0].MinimumVoltage
	vmax := calRanges[0].MaximumVoltage
	voltScale := (vmax - vmin) / 4095.0
	voltOffset := (vmax + vmin) / 2.0

	analogData, err := readZipFile(r, "analog-0.bin")
	if err != nil {
		return nil, fmt.Errorf("no analog data found in .sal file: %w", err)
	}

	return parseAnalogBin(analogData, sampleRate, voltScale, voltOffset)
}

// parseAnalogBin parses the Saleae Logic 2 analog binary format.
//
// Format:
//   - File header: 8 bytes magic "<SALEAE>" + 4 bytes version (u32 LE) + 4 bytes type (u32 LE)
//     + variable additional fields = 51 bytes total
//   - Then repeating segments, each containing:
//   - Segment header: 3 x uint64 LE [begin_sample, next_begin, delta]
//   - Sample data: (next_begin - begin_sample) x uint16 LE ADC values
//   - Mipmap chain: first level has 28-byte header, subsequent levels have 8-byte headers
//     Each level stores pair_count min/max pairs (pair_count*4 bytes of uint16 data)
//     Levels halve until pair_count reaches 0
func parseAnalogBin(data []byte, sampleRate, voltScale, voltOffset float64) ([]decoder.Sample, error) {
	if len(data) < 51+24 {
		return nil, fmt.Errorf("analog data too small (%d bytes)", len(data))
	}

	// Verify magic
	magic := string(data[0:8])
	if magic != "<SALEAE>" {
		return nil, fmt.Errorf("invalid analog data magic: %q", magic)
	}

	var samples []decoder.Sample
	pos := 51 // Skip file header

	for pos+24 <= len(data) {
		// Read segment header: 3 x uint64 LE
		beginSample := binary.LittleEndian.Uint64(data[pos:])
		nextBegin := binary.LittleEndian.Uint64(data[pos+8:])
		pos += 24

		sampleCount := int(nextBegin - beginSample)
		if sampleCount <= 0 || sampleCount > 10_000_000 {
			break
		}

		// Read ADC samples
		bytesNeeded := sampleCount * 2
		if pos+bytesNeeded > len(data) {
			break
		}

		// Downsample: keep every Nth sample to reduce memory and processing.
		// Manchester half-bit is ~500µs; at 1.5625 MHz that's ~781 samples per half-bit.
		// Keeping every 10th sample still gives ~78 samples per half-bit — plenty for edge detection.
		step := int(sampleRate / 150000)
		if step < 1 {
			step = 1
		}
		for i := 0; i < sampleCount; i += step {
			adc := binary.LittleEndian.Uint16(data[pos+i*2:])
			sampleIdx := int(beginSample) + i
			t := float64(sampleIdx) / sampleRate
			voltage := float64(adc)*voltScale + voltOffset
			samples = append(samples, decoder.Sample{Time: t, Voltage: voltage})
		}
		pos += bytesNeeded

		// Skip mipmap chain
		if pos+28 > len(data) {
			break
		}

		// First mipmap level: 28-byte header
		pairCount := int(binary.LittleEndian.Uint32(data[pos:]))
		pos += 28

		if pairCount > 0 {
			pos += pairCount * 4 // min/max pairs as uint16

			// Subsequent levels: 8-byte header + data, pair count halves
			for pairCount > 1 {
				pairCount = pairCount / 2
				if pos+8+pairCount*4 > len(data) {
					return samples, nil
				}
				pos += 8 + pairCount*4
			}
		}

		// Skip 8-byte terminator block (pair_count=0)
		if pos+8 <= len(data) {
			check := binary.LittleEndian.Uint32(data[pos:])
			if check == 0 {
				pos += 8
			}
		}
	}

	if len(samples) == 0 {
		return nil, fmt.Errorf("no samples found in analog data")
	}

	return samples, nil
}

func readZipFile(r *zip.ReadCloser, name string) ([]byte, error) {
	for _, f := range r.File {
		if f.Name == name {
			rc, err := f.Open()
			if err != nil {
				return nil, err
			}
			defer rc.Close()
			return io.ReadAll(rc)
		}
	}
	return nil, fmt.Errorf("file %q not found in archive", name)
}
